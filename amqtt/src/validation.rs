use std::time::SystemTime;
use log::{info, error, warn};

use rustls::{
    Certificate,
    client::ServerCertVerified, Error
};

use futures_rustls::rustls::ServerName;

use x509_parser::prelude::*;

#[derive(Debug)]
pub enum EndEntityOrCa<'a> {
    EndEntity,
    Ca(&'a Cert<'a>),
}

#[derive(Debug)]
pub struct Cert<'a> {
    inner: &'a X509Certificate<'a>,
    ee_or_ca: EndEntityOrCa<'a>
}

impl<'a> Cert<'a> {
    pub fn end_entity(cert: &'a X509Certificate) -> Self {
        Cert {
            inner: cert,
            ee_or_ca: EndEntityOrCa::EndEntity
        }
    }

    pub fn parse_cert(cert: &'a X509Certificate, ca_cert: &'a Cert) -> Self {
        Cert {
            inner: cert,
            ee_or_ca: EndEntityOrCa::Ca(ca_cert)
        }
    }

    pub fn used_as_ca(&self) -> bool {
        match self.ee_or_ca {
            EndEntityOrCa::EndEntity => false,
            EndEntityOrCa::Ca(_) => true
        }
    }

    fn inner(&self) -> &'a X509Certificate {
        self.inner
    }
}

#[derive(Debug)]
pub struct SelfSignedCertVerifier {
    raw_certs: Vec<u8>,
    pems: Vec<Pem>,
}

impl SelfSignedCertVerifier {
    pub fn new(root_cert: &[u8]) -> Self {
        let raw_certs = root_cert.to_vec();

        let pems: Vec<Pem> = Pem::iter_from_buffer(&raw_certs)
            .filter_map(|pem_res| {
                if let Ok(pem) = pem_res {
                    // Even though we only return Pem, this is a good place to
                    // check that cert is parsable so that we can just unwrap all calls later
                    if pem.parse_x509().is_ok() {
                        return Some(pem);
                    }
                    warn!("Root DER decoding failed in constructor");
                }
                else {
                    warn!("Root PEM decoding failed in constructor");
                }
                None
            })
            .collect();

        Self {
            raw_certs,
            pems
        }
    }

    fn verify_server_name(cert: &X509Certificate, server_name: &ServerName) -> Result<(), Error> {
        let mut subject_alternative_names = cert.extensions().iter().filter_map(|extension|{
            if let ParsedExtension::SubjectAlternativeName(san) = extension.parsed_extension(){
                Some(san)
            }
            else {
                None
            }
        });

        if let ServerName::DnsName(server) = server_name {
            if let Some(subject_names) = &subject_alternative_names.next() {
                for name in &subject_names.general_names {
                    let s = match name {
                        GeneralName::DNSName(s) => {
                            *s
                        },
                        GeneralName::IPAddress(_b) => {
                            warn!("IP-address parsing not implemented");
                            ""
                        },
                        _ => {
                            warn!("Invalid SAN type {:?}", name);
                            ""
                        }
                    };

                    if s == server.as_ref() {
                        info!("Found server name:{} for SAN", s);
                        return Ok(());
                    }
                }
                error!("No right server names were found from SAN");
            }
            else {
                warn!("Didn't find SAN element. Checking subject");
                let matches_found: Vec::<_> = cert
                    .subject()
                    .iter_common_name()
                    .filter(|name| {
                        if let ServerName::DnsName(server) = server_name {
                            if let Ok(cert_server_str) = name.as_str() {
                                info!("Found server name {} from cert", cert_server_str);
                                return cert_server_str == server.as_ref()
                            }
                        }
                        false
                    })
                    .collect();

                if !matches_found.is_empty() {
                    return Ok(());
                }
                else {
                    error!("No DN match with subject found");
                }
            }
        }
        else {
            panic!("DNS server enum should always be there");
        }
        return Err(Error::InvalidCertificateData(String::from("No DN match found")));
    }

    fn verify_chain(anchor_pems: &Vec<Pem>, cert: &Cert, intermediate_certs: &[&[u8]], sub_ca_count: usize) -> Result<(), Error> {
        SelfSignedCertVerifier::verify_cert_info(cert.inner())?;

        let used_as_ca = cert.used_as_ca();

        if used_as_ca {
            const MAX_SUB_CA_COUNT: usize = 6;

            if sub_ca_count >= MAX_SUB_CA_COUNT {
                error!("Hitting max count limit");
                return Err(Error::General(String::from("Unknown Issuer")));
            }
        }
        else {
            assert_eq!(0, sub_ca_count);
        }

        for pem in anchor_pems {
            let x509 = pem.parse_x509().unwrap();

            let issuer = cert.inner().issuer();
            if issuer != x509.subject() {
                continue;
            }

            info!("Matching issuer {} found", issuer);

            // TODO: Check anchor signatures
            return Ok(())
        }

        loop_while_non_fatal_error(intermediate_certs, |pem| {
            //for &pem in intermediate_certs {
            let (_rem, potential_cert) = X509Certificate::from_der(pem).unwrap();
            let potential_issuer = Cert::parse_cert(&potential_cert, cert);

            if potential_issuer.inner().subject() != cert.inner().issuer() {
                warn!("Subject didn't match when processing intermediates");
                return Err(Error::General(String::from("Unknown Issuer")));
                //continue;
            }

            // Prevent loops; see RFC 4158 section 5.2.
            let mut prev = cert;
            loop {
                if potential_issuer.inner().tbs_certificate.subject_pki == prev.inner().tbs_certificate.subject_pki
                    && potential_issuer.inner().subject() == prev.inner().subject() {
                    //return Err(Error::UnknownIssuer);
                    warn!("Subject didn't match in loop first stage");
                    return Err(Error::General(String::from("Unknown Issuer")));
                }

                if let EndEntityOrCa::Ca(ca_cert) = prev.ee_or_ca {
                    prev = ca_cert;
                    continue;
                }
                break;
            }

            let next_sub_ca_count = if used_as_ca {
                sub_ca_count + 1
            }
            else {
                sub_ca_count
            };

            SelfSignedCertVerifier::verify_chain(anchor_pems, &potential_issuer, intermediate_certs, next_sub_ca_count)
        })

        // We didn't find the end of the chain so
        // TODO: Add intermediate chain building here
        // We only end up here if the certificate signature didn't match
        //error!("Didn't find candidates from intermediate certificates");
        //Err(Error::General(String::from("Unknown Issuer")))
    }

    fn verify_cert_info(x509: &X509Certificate) -> Result<(), Error> {
        info!("X.509 Subject: {}", x509.subject());
        info!("X.509 Version: {}", x509.version());
        info!("X.509 Issuer: {}", x509.issuer());
        info!("X.509 serial: {}", x509.tbs_certificate.raw_serial_as_string());

        // Starting with timing
        if !x509.validity().is_valid() {
            error!("Outside of validity period");
            return Err(Error::InvalidCertificateData(String::from("Invalid time information")));
        }

        // TODO: SAN validation should be added
        if x509.version() != X509Version::V3 {
            warn!("Certificate version is not V3 but {}", x509.version());
        }
        Ok(())
    }
}

impl rustls::client::ServerCertVerifier for SelfSignedCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &Certificate,
        intermediates: &[Certificate],
        server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime
    ) -> Result<ServerCertVerified, Error> {
        let intermediates: Vec<&[u8]> = intermediates
            .iter()
            .map(|cert| cert.0.as_ref())
            .collect();

        info!("Server-name {:?} and {} intermediate certificates", server_name, intermediates.len());

        if let Ok((_rem, x509)) = X509Certificate::from_der(&end_entity.0) {
            let cert = Cert::end_entity(&x509);
            SelfSignedCertVerifier::verify_chain(&self.pems, &cert, &intermediates, 0)?;
            info!("Checking root certificate");

            // TODO: Verify policy (if needed)

            SelfSignedCertVerifier::verify_server_name(&x509, server_name)?;

            return Ok(rustls::client::ServerCertVerified::assertion());
        }
        else {
            error!("Certificate parsing failed");
        }

        return Err(Error::InvalidCertificateData(String::from("No valid DN found for issuer")));
    }

    /* We can use the default implementations for signature validation
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        cert: &Certificate,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        debug!("verify_tls12_signature");

        // This certificate has been already validated so unwrapping is 'safe'
        let (_rem, x509) = X509Certificate::from_der(&cert.0).unwrap();
        return self.check_signature(&x509);
    }


    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        cert: &Certificate,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        debug!("verify_tls13_signature");

        // This certificate has been already validated so unwrapping is 'safe'
        let (_rem, x509) = X509Certificate::from_der(&cert.0).unwrap();
        self.check_signature(&x509)
    }*/
}

// Borrowed from Webpki
fn loop_while_non_fatal_error<V>(
    values: V,
    f: impl Fn(V::Item) -> Result<(), Error>,
) -> Result<(), Error>
where
    V: IntoIterator,
{
    for v in values {
        match f(v) {
            Ok(()) => {
                return Ok(());
            }
            Err(..) => {
                // If the error is not fatal, then keep going.
            }
        }
    }
    Err(Error::General(String::from("Unknown Issuer")))
}