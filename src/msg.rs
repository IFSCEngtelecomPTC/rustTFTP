enum Mensagem {
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
    fn serialize() -> Vec<u8>;    
}

// Mensagens de requisição, que podem ser RRQ ou WRQ
enum TipoReq {
    WRQ,
    RRQ
}

enum Modo {
    Netascii,
    Octet,
    Mail
}

pub struct Requisicao {
    fname: String,
    modo: Modo,
    tipo: TipoReq
}

// Mensagem de dados
pub struct DATA {
    block: u16,
    body: Vec<u8>
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
    fn serialize() -> Vec<u8> {
        vec![]
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
