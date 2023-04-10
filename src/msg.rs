use std::fmt;

pub enum Mensagem {
    Wrq(Requisicao), 
    Rrq(Requisicao),
    Err(ERR),
    Data(DATA),
    Ack(ACK)
}

fn get_shortint(buffer: &[u8]) -> u16 {
    u16::from_be_bytes([buffer[0], buffer[1]])
}

pub trait Codec {
    fn init(&self, code: u16) -> bytes::BytesMut {
        let buffer = bytes::BytesMut::from(code.to_be_bytes().as_ref());
        buffer
    }

    fn serialize(&self) -> bytes::BytesMut;    
}

// Mensagens de requisição, que podem ser RRQ ou WRQ
pub enum TipoReq {
    WRQ,
    RRQ
}

impl TipoReq {
    fn code(&self) -> u16 {
        match self {
            TipoReq::RRQ => Requisicao::CODE_RRQ,
            TipoReq::WRQ => Requisicao::CODE_WRQ
        }
    }
}

#[derive(Debug)]
pub enum Modo {
    Netascii,
    Octet,
    Mail
}

impl Modo {
    fn as_str0(&self) -> String {
        String::from(match self {
            Modo::Mail => "mail",
            Modo::Netascii => "netascii",
            Modo::Octet => "octet"
        })
    }

    fn as_str(&self) -> &str {
        match self {
            Modo::Mail => "mail",
            Modo::Netascii => "netascii",
            Modo::Octet => "octet"
        }
    }

}
pub struct Requisicao {
    pub fname: String,
    pub modo: Modo,
    pub tipo: TipoReq
}

// Mensagem de dados
pub struct DATA {
    pub block: u16,
    pub body: Vec<u8>
}

// Mensagem de confirmação
pub struct ACK {
    block: u16
}

// Mensagem de erro
pub struct ERR {
    err_code: u16,
    err_msg: String
}


// Implementação do trait Codec
impl Codec for Requisicao {
    fn serialize(&self) -> bytes::BytesMut {
        let mut buffer = self.init(self.tipo.code());
        buffer.extend(self.fname.as_bytes());
        buffer.extend(&[0]);
        buffer.extend(self.modo.as_str().as_bytes());
        buffer.extend(&[0]);
        buffer
    }
}

impl Codec for ACK {
    fn serialize(&self) -> bytes::BytesMut {
        let mut buffer = self.init(ACK::CODE);
        buffer.extend(self.block.to_be_bytes());
        buffer
    }
}

impl Codec for DATA {
    fn serialize(&self) -> bytes::BytesMut {
        let mut buffer = self.init(DATA::CODE);
        buffer.extend(self.block.to_be_bytes());
        buffer.extend(&self.body);
        buffer
    }
}

impl Codec for ERR {
    fn serialize(&self) ->bytes::BytesMut {
        let mut buffer = self.init(DATA::CODE);
        buffer.extend(self.err_code.to_be_bytes());
        buffer.extend(self.err_msg.as_bytes());
        buffer.extend(&[0]);
        buffer
    }
}

fn get_string(buffer: &[u8]) -> String {
    let sub:Vec<u8> = buffer.into_iter()
                            .take_while(|x| **x != 0)
                            .map(|x| *x).collect();
    String::from_utf8_lossy(&sub).into_owned()
}

impl Requisicao {
    const CODE_RRQ:u16 = 1;
    const CODE_WRQ:u16 = 2;

    pub fn from_bytes(buffer: Vec<u8>) -> Option<Self> {
        let opcode:u16 = get_shortint(&buffer);
        let tipo = match opcode {
            Requisicao::CODE_RRQ => TipoReq::RRQ,
            Requisicao::CODE_WRQ => TipoReq::WRQ,
            _ => {
                return None;
            }
        };
        let name = get_string(&buffer[2..]);
        let modo = get_string(&buffer[name.len()+3..]);
        Some(Requisicao{
            fname: name, 
            modo: match modo.as_str() {
                "octet" => Modo::Octet,
                "netascii" => Modo::Netascii,
                "mail" => Modo::Mail,
                _ => {
                    panic!("modo inválido");
                }
            },
            tipo: tipo
        })
    }

    pub fn new(tipo: TipoReq, fname: &str, modo: Modo) -> Option<Self> {
        if fname.is_empty() {
            return None;
        }
        Some(Requisicao {
            fname: fname.to_owned(),
            modo: modo,
            tipo: tipo
        })
    }

    pub fn new_wrq(fname: &str, modo: Modo) -> Option<Self> {
        Requisicao::new(TipoReq::WRQ, fname, modo)
    }
    pub fn new_rrq(fname: &str, modo: Modo) -> Option<Self> {
        Requisicao::new(TipoReq::RRQ, fname, modo)
    }

}

impl DATA {
    const CODE:u16 = 3;
    pub const SIZE:usize = 512;

    pub fn from_bytes(buffer: Vec<u8>) -> Option<Self> {
        let opcode:u16 = get_shortint(&buffer);
        if opcode != DATA::CODE {
            return None;
        }
        let blocknum = get_shortint(&buffer[2..]);
        Some(DATA {
            block: blocknum,
            body: buffer[4..].to_vec()
        })
    }

    pub fn new(blocknum: u16, buffer: &[u8]) -> Option<Self> {
        if blocknum < 1 {
            return None;
        }
        Some(DATA {
            block: blocknum,
            body: buffer.to_vec()
        })
    }
}

impl ACK {
    const CODE:u16 = 4;

    pub fn from_bytes(buffer: Vec<u8>) -> Option<Self> {
        let opcode:u16 = get_shortint(&buffer);
        if opcode != ACK::CODE {
            return None;
        }
        let blocknum = get_shortint(&buffer[2..]);
        Some(ACK {
            block: blocknum,
        })
    }
    pub fn new(blocknum: u16) -> Option<Self> {
        if blocknum < 1 {
            return None;
        }
        Some(ACK {
            block: blocknum,
        })
    }   
}

impl ERR {
    const CODE:u16 = 5;

    pub fn from_bytes(buffer: Vec<u8>) -> Option<Self> {
        let opcode:u16 = get_shortint(&buffer);
        if opcode != ERR::CODE {
            return None
        }
        let err_code = get_shortint(&buffer[2..]);

        let err_msg = get_string(&buffer[4..]);
        Some(ERR{
            err_code: err_code,
            err_msg: err_msg
        })
    }

    pub fn new(err_code: u16, err_msg: &str) -> Option<Self> {
        Some(ERR {
            err_code: err_code,
            err_msg: err_msg.to_owned()
        })
    }   

}

/// A factory function to build a TFTP message from a vector of bytes
pub fn from_bytes(buffer: Vec<u8>) -> Option<Mensagem> {
    let opcode:u16 = get_shortint(&buffer);
    match opcode {
        Requisicao::CODE_RRQ => Requisicao::from_bytes(buffer).and_then(|m| Some(Mensagem::Rrq(m))),
        Requisicao::CODE_WRQ => Requisicao::from_bytes(buffer).and_then(|m| Some(Mensagem::Wrq(m))),
        DATA::CODE => DATA::from_bytes(buffer).and_then(|m| Some(Mensagem::Data(m))),
        ACK::CODE => ACK::from_bytes(buffer).and_then(|m| Some(Mensagem::Ack(m))),
        ERR::CODE => ERR::from_bytes(buffer).and_then(|m| Some(Mensagem::Err(m))),
        _ => None
    }
}

impl fmt::Display for Requisicao {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tipo = match self.tipo {
            TipoReq::RRQ => {
                "RRQ"
            },
            TipoReq::WRQ => {
                "WRQ"
            }
        };
        write!(f, "{}: filename={}, modo={:?}", tipo, self.fname, self.modo)
    }    
}

impl fmt::Display for DATA {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Data: blocknum={}, body len={}", self.block, self.body.len())
    }    
}

impl fmt::Display for ACK {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ACK: blocknum={}", self.block)
    }    
}

impl fmt::Display for ERR {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Err: err_code={}, err_msg={}", self.err_code, self.err_msg)
    }    
}

impl fmt::Display for Mensagem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mensagem::Rrq(msg) => write!(f, "{}", msg),
            Mensagem::Wrq(msg) => write!(f, "{}", msg),
            Mensagem::Data(msg) => write!(f, "{}", msg),
            Mensagem::Ack(msg) => write!(f, "{}", msg),
            Mensagem::Err(msg) => write!(f, "{}", msg)
        }
    }    
}