import datetime

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.backends import default_backend

from cryptography import x509
from cryptography.x509.oid import NameOID
from cryptography.hazmat.primitives import hashes

# generate private/public key pair
class keys(object):
    """
    Class for generating the keys and certificates. Only EC supported now
    """
    def __init__(self, country:str, province:str, locality_name: str, organization: str, common_name:str, validity_in_days: int):
        self._key = ec.generate_private_key(backend=default_backend(), curve=ec.SECP256R1())

        self._country = country
        self._province = province
        self._locality = locality_name
        self._organization = organization
        self._common_name = common_name
        self._validity = validity_in_days


    @property
    def private_pem(self):
        if not hasattr(self, "_private_key"):
            self._private_key = self._key.private_bytes(
                encoding=serialization.Encoding.PEM,
                format=serialization.PrivateFormat.TraditionalOpenSSL,
                encryption_algorithm=serialization.NoEncryption()
                )

        return self._private_key


    @property
    def public_pem(self):
        if not hasattr(self, "_public_key"):
            pub = self._key.public_key()
            self._public_key = pub.public_bytes(serialization.Encoding.PEM, serialization.PublicFormat.SubjectPublicKeyInfo)

        return self._public_key

    @property
    def public_x509_pem(self):
        if not hasattr(self, '_x509_pem'):
            subject = issuer = x509.Name([
                x509.NameAttribute(NameOID.COUNTRY_NAME, self._country),
                x509.NameAttribute(NameOID.STATE_OR_PROVINCE_NAME, self._province),
                x509.NameAttribute(NameOID.LOCALITY_NAME, self._locality),
                x509.NameAttribute(NameOID.ORGANIZATION_NAME, self._organization),
                x509.NameAttribute(NameOID.COMMON_NAME, self._common_name),
            ])

            cert = x509.CertificateBuilder().subject_name(
                subject
            ).issuer_name(
                issuer
            ).public_key(
                self._key.public_key()
            ).serial_number(
                x509.random_serial_number()
            ).not_valid_before(
                datetime.datetime.utcnow()
            ).not_valid_after(
                datetime.datetime.utcnow() + datetime.timedelta(days=self._validity)
            ).add_extension(
                x509.SubjectAlternativeName([x509.DNSName(u"localhost")]),
                critical=False,
            # Sign our certificate with our private key
            ).sign(self._key, hashes.SHA256(), default_backend())

            self._x509_pem = cert.public_bytes(serialization.Encoding.PEM)
        return self._x509_pem
