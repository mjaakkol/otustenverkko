// This file is generated by rust-protobuf 2.22.1. Do not edit
// @generated

// https://github.com/rust-lang/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![allow(unused_attributes)]
#![cfg_attr(rustfmt, rustfmt::skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unused_imports)]
#![allow(unused_results)]
//! Generated file from `messages.proto`

/// Generated files are compatible only with the same version
/// of protobuf runtime.
// const _PROTOBUF_VERSION_CHECK: () = ::protobuf::VERSION_2_22_1;

#[derive(PartialEq,Clone,Default)]
pub struct EnvironmentData {
    // message fields
    pub time: u32,
    pub temperature_k: u32,
    pub humidity_rh: u32,
    pub voc_index: u32,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a EnvironmentData {
    fn default() -> &'a EnvironmentData {
        <EnvironmentData as ::protobuf::Message>::default_instance()
    }
}

impl EnvironmentData {
    pub fn new() -> EnvironmentData {
        ::std::default::Default::default()
    }

    // uint32 time = 1;


    pub fn get_time(&self) -> u32 {
        self.time
    }
    pub fn clear_time(&mut self) {
        self.time = 0;
    }

    // Param is passed by value, moved
    pub fn set_time(&mut self, v: u32) {
        self.time = v;
    }

    // uint32 temperature_k = 2;


    pub fn get_temperature_k(&self) -> u32 {
        self.temperature_k
    }
    pub fn clear_temperature_k(&mut self) {
        self.temperature_k = 0;
    }

    // Param is passed by value, moved
    pub fn set_temperature_k(&mut self, v: u32) {
        self.temperature_k = v;
    }

    // uint32 humidity_rh = 3;


    pub fn get_humidity_rh(&self) -> u32 {
        self.humidity_rh
    }
    pub fn clear_humidity_rh(&mut self) {
        self.humidity_rh = 0;
    }

    // Param is passed by value, moved
    pub fn set_humidity_rh(&mut self, v: u32) {
        self.humidity_rh = v;
    }

    // uint32 voc_index = 4;


    pub fn get_voc_index(&self) -> u32 {
        self.voc_index
    }
    pub fn clear_voc_index(&mut self) {
        self.voc_index = 0;
    }

    // Param is passed by value, moved
    pub fn set_voc_index(&mut self, v: u32) {
        self.voc_index = v;
    }
}

impl ::protobuf::Message for EnvironmentData {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint32()?;
                    self.time = tmp;
                },
                2 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint32()?;
                    self.temperature_k = tmp;
                },
                3 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint32()?;
                    self.humidity_rh = tmp;
                },
                4 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint32()?;
                    self.voc_index = tmp;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if self.time != 0 {
            my_size += ::protobuf::rt::value_size(1, self.time, ::protobuf::wire_format::WireTypeVarint);
        }
        if self.temperature_k != 0 {
            my_size += ::protobuf::rt::value_size(2, self.temperature_k, ::protobuf::wire_format::WireTypeVarint);
        }
        if self.humidity_rh != 0 {
            my_size += ::protobuf::rt::value_size(3, self.humidity_rh, ::protobuf::wire_format::WireTypeVarint);
        }
        if self.voc_index != 0 {
            my_size += ::protobuf::rt::value_size(4, self.voc_index, ::protobuf::wire_format::WireTypeVarint);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if self.time != 0 {
            os.write_uint32(1, self.time)?;
        }
        if self.temperature_k != 0 {
            os.write_uint32(2, self.temperature_k)?;
        }
        if self.humidity_rh != 0 {
            os.write_uint32(3, self.humidity_rh)?;
        }
        if self.voc_index != 0 {
            os.write_uint32(4, self.voc_index)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        Self::descriptor_static()
    }

    fn new() -> EnvironmentData {
        EnvironmentData::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint32>(
                "time",
                |m: &EnvironmentData| { &m.time },
                |m: &mut EnvironmentData| { &mut m.time },
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint32>(
                "temperature_k",
                |m: &EnvironmentData| { &m.temperature_k },
                |m: &mut EnvironmentData| { &mut m.temperature_k },
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint32>(
                "humidity_rh",
                |m: &EnvironmentData| { &m.humidity_rh },
                |m: &mut EnvironmentData| { &mut m.humidity_rh },
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint32>(
                "voc_index",
                |m: &EnvironmentData| { &m.voc_index },
                |m: &mut EnvironmentData| { &mut m.voc_index },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<EnvironmentData>(
                "EnvironmentData",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static EnvironmentData {
        static instance: ::protobuf::rt::LazyV2<EnvironmentData> = ::protobuf::rt::LazyV2::INIT;
        instance.get(EnvironmentData::new)
    }
}

impl ::protobuf::Clear for EnvironmentData {
    fn clear(&mut self) {
        self.time = 0;
        self.temperature_k = 0;
        self.humidity_rh = 0;
        self.voc_index = 0;
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for EnvironmentData {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for EnvironmentData {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct EnvironmentDataBlocks {
    // message fields
    pub blocks: ::protobuf::RepeatedField<EnvironmentData>,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a EnvironmentDataBlocks {
    fn default() -> &'a EnvironmentDataBlocks {
        <EnvironmentDataBlocks as ::protobuf::Message>::default_instance()
    }
}

impl EnvironmentDataBlocks {
    pub fn new() -> EnvironmentDataBlocks {
        ::std::default::Default::default()
    }

    // repeated .luxilma.EnvironmentData blocks = 1;


    pub fn get_blocks(&self) -> &[EnvironmentData] {
        &self.blocks
    }
    pub fn clear_blocks(&mut self) {
        self.blocks.clear();
    }

    // Param is passed by value, moved
    pub fn set_blocks(&mut self, v: ::protobuf::RepeatedField<EnvironmentData>) {
        self.blocks = v;
    }

    // Mutable pointer to the field.
    pub fn mut_blocks(&mut self) -> &mut ::protobuf::RepeatedField<EnvironmentData> {
        &mut self.blocks
    }

    // Take field
    pub fn take_blocks(&mut self) -> ::protobuf::RepeatedField<EnvironmentData> {
        ::std::mem::replace(&mut self.blocks, ::protobuf::RepeatedField::new())
    }
}

impl ::protobuf::Message for EnvironmentDataBlocks {
    fn is_initialized(&self) -> bool {
        for v in &self.blocks {
            if !v.is_initialized() {
                return false;
            }
        };
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_repeated_message_into(wire_type, is, &mut self.blocks)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        for value in &self.blocks {
            let len = value.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
        };
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        for v in &self.blocks {
            os.write_tag(1, ::protobuf::wire_format::WireTypeLengthDelimited)?;
            os.write_raw_varint32(v.get_cached_size())?;
            v.write_to_with_cached_sizes(os)?;
        };
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        Self::descriptor_static()
    }

    fn new() -> EnvironmentDataBlocks {
        EnvironmentDataBlocks::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_repeated_field_accessor::<_, ::protobuf::types::ProtobufTypeMessage<EnvironmentData>>(
                "blocks",
                |m: &EnvironmentDataBlocks| { &m.blocks },
                |m: &mut EnvironmentDataBlocks| { &mut m.blocks },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<EnvironmentDataBlocks>(
                "EnvironmentDataBlocks",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static EnvironmentDataBlocks {
        static instance: ::protobuf::rt::LazyV2<EnvironmentDataBlocks> = ::protobuf::rt::LazyV2::INIT;
        instance.get(EnvironmentDataBlocks::new)
    }
}

impl ::protobuf::Clear for EnvironmentDataBlocks {
    fn clear(&mut self) {
        self.blocks.clear();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for EnvironmentDataBlocks {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for EnvironmentDataBlocks {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct Configuration {
    // message fields
    pub sample_block_size: u32,
    pub target_firmware_version: u32,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a Configuration {
    fn default() -> &'a Configuration {
        <Configuration as ::protobuf::Message>::default_instance()
    }
}

impl Configuration {
    pub fn new() -> Configuration {
        ::std::default::Default::default()
    }

    // uint32 sample_block_size = 1;


    pub fn get_sample_block_size(&self) -> u32 {
        self.sample_block_size
    }
    pub fn clear_sample_block_size(&mut self) {
        self.sample_block_size = 0;
    }

    // Param is passed by value, moved
    pub fn set_sample_block_size(&mut self, v: u32) {
        self.sample_block_size = v;
    }

    // uint32 target_firmware_version = 2;


    pub fn get_target_firmware_version(&self) -> u32 {
        self.target_firmware_version
    }
    pub fn clear_target_firmware_version(&mut self) {
        self.target_firmware_version = 0;
    }

    // Param is passed by value, moved
    pub fn set_target_firmware_version(&mut self, v: u32) {
        self.target_firmware_version = v;
    }
}

impl ::protobuf::Message for Configuration {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint32()?;
                    self.sample_block_size = tmp;
                },
                2 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint32()?;
                    self.target_firmware_version = tmp;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if self.sample_block_size != 0 {
            my_size += ::protobuf::rt::value_size(1, self.sample_block_size, ::protobuf::wire_format::WireTypeVarint);
        }
        if self.target_firmware_version != 0 {
            my_size += ::protobuf::rt::value_size(2, self.target_firmware_version, ::protobuf::wire_format::WireTypeVarint);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if self.sample_block_size != 0 {
            os.write_uint32(1, self.sample_block_size)?;
        }
        if self.target_firmware_version != 0 {
            os.write_uint32(2, self.target_firmware_version)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        Self::descriptor_static()
    }

    fn new() -> Configuration {
        Configuration::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint32>(
                "sample_block_size",
                |m: &Configuration| { &m.sample_block_size },
                |m: &mut Configuration| { &mut m.sample_block_size },
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeUint32>(
                "target_firmware_version",
                |m: &Configuration| { &m.target_firmware_version },
                |m: &mut Configuration| { &mut m.target_firmware_version },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<Configuration>(
                "Configuration",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static Configuration {
        static instance: ::protobuf::rt::LazyV2<Configuration> = ::protobuf::rt::LazyV2::INIT;
        instance.get(Configuration::new)
    }
}

impl ::protobuf::Clear for Configuration {
    fn clear(&mut self) {
        self.sample_block_size = 0;
        self.target_firmware_version = 0;
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Configuration {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Configuration {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

static file_descriptor_proto_data: &'static [u8] = b"\
    \n\x0emessages.proto\x12\x07luxilma\"\x88\x01\n\x0fEnvironmentData\x12\
    \x12\n\x04time\x18\x01\x20\x01(\rR\x04time\x12#\n\rtemperature_k\x18\x02\
    \x20\x01(\rR\x0ctemperatureK\x12\x1f\n\x0bhumidity_rh\x18\x03\x20\x01(\r\
    R\nhumidityRh\x12\x1b\n\tvoc_index\x18\x04\x20\x01(\rR\x08vocIndex\"I\n\
    \x15EnvironmentDataBlocks\x120\n\x06blocks\x18\x01\x20\x03(\x0b2\x18.lux\
    ilma.EnvironmentDataR\x06blocks\"s\n\rConfiguration\x12*\n\x11sample_blo\
    ck_size\x18\x01\x20\x01(\rR\x0fsampleBlockSize\x126\n\x17target_firmware\
    _version\x18\x02\x20\x01(\rR\x15targetFirmwareVersionb\x06proto3\
";

static file_descriptor_proto_lazy: ::protobuf::rt::LazyV2<::protobuf::descriptor::FileDescriptorProto> = ::protobuf::rt::LazyV2::INIT;

fn parse_descriptor_proto() -> ::protobuf::descriptor::FileDescriptorProto {
    ::protobuf::Message::parse_from_bytes(file_descriptor_proto_data).unwrap()
}

pub fn file_descriptor_proto() -> &'static ::protobuf::descriptor::FileDescriptorProto {
    file_descriptor_proto_lazy.get(|| {
        parse_descriptor_proto()
    })
}
