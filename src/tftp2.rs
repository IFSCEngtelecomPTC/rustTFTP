#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Req {
    #[prost(string, required, tag = "1")]
    pub fname: ::prost::alloc::string::String,
    #[prost(enumeration = "Mode", required, tag = "2")]
    pub mode: i32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Data {
    #[prost(bytes = "vec", required, tag = "1")]
    pub message: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, required, tag = "2")]
    pub block_n: u32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ack {
    #[prost(uint32, required, tag = "1")]
    pub block_n: u32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Error {
    #[prost(enumeration = "ErrorCode", required, tag = "1")]
    pub errorcode: i32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Path {
    #[prost(string, required, tag = "1")]
    pub path: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListResponse {
    #[prost(message, repeated, tag = "1")]
    pub items: ::prost::alloc::vec::Vec<ListItem>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListItem {
    #[prost(oneof = "list_item::Answer", tags = "1, 2")]
    pub answer: ::core::option::Option<list_item::Answer>,
}
/// Nested message and enum types in `ListItem`.
pub mod list_item {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Answer {
        #[prost(message, tag = "1")]
        File(super::File),
        #[prost(message, tag = "2")]
        Dir(super::Path),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct File {
    #[prost(string, required, tag = "1")]
    pub nome: ::prost::alloc::string::String,
    #[prost(int32, required, tag = "2")]
    pub tamanho: i32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Move {
    #[prost(string, required, tag = "1")]
    pub nome_orig: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "2")]
    pub nome_novo: ::core::option::Option<::prost::alloc::string::String>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Mensagem {
    #[prost(oneof = "mensagem::Msg", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9")]
    pub msg: ::core::option::Option<mensagem::Msg>,
}
/// Nested message and enum types in `Mensagem`.
pub mod mensagem {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Msg {
        #[prost(message, tag = "1")]
        Rrq(super::Req),
        #[prost(message, tag = "2")]
        Wrq(super::Req),
        #[prost(message, tag = "3")]
        Data(super::Data),
        #[prost(message, tag = "4")]
        Ack(super::Ack),
        #[prost(message, tag = "5")]
        Error(super::Error),
        #[prost(message, tag = "6")]
        List(super::Path),
        #[prost(message, tag = "7")]
        ListResp(super::ListResponse),
        #[prost(message, tag = "8")]
        Mkdir(super::Path),
        #[prost(message, tag = "9")]
        Move(super::Move),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Mode {
    Netascii = 1,
    Octet = 2,
    Mail = 3,
}
impl Mode {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Mode::Netascii => "netascii",
            Mode::Octet => "octet",
            Mode::Mail => "mail",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "netascii" => Some(Self::Netascii),
            "octet" => Some(Self::Octet),
            "mail" => Some(Self::Mail),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ErrorCode {
    FileNotFound = 1,
    AccessViolation = 2,
    DiskFull = 3,
    IllegalOperation = 4,
    UnknownTid = 5,
    FileExists = 6,
    UnknownSession = 7,
    Unedfined = 8,
}
impl ErrorCode {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ErrorCode::FileNotFound => "FileNotFound",
            ErrorCode::AccessViolation => "AccessViolation",
            ErrorCode::DiskFull => "DiskFull",
            ErrorCode::IllegalOperation => "IllegalOperation",
            ErrorCode::UnknownTid => "UnknownTid",
            ErrorCode::FileExists => "FileExists",
            ErrorCode::UnknownSession => "UnknownSession",
            ErrorCode::Unedfined => "Unedfined",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "FileNotFound" => Some(Self::FileNotFound),
            "AccessViolation" => Some(Self::AccessViolation),
            "DiskFull" => Some(Self::DiskFull),
            "IllegalOperation" => Some(Self::IllegalOperation),
            "UnknownTid" => Some(Self::UnknownTid),
            "FileExists" => Some(Self::FileExists),
            "UnknownSession" => Some(Self::UnknownSession),
            "Unedfined" => Some(Self::Unedfined),
            _ => None,
        }
    }
}
