enum Mensagem {
    wrq(WRQ), 
    rrq(RRQ),
    err(Err),
    data(Data),
    ack(Ack)
}

fn get_opcode(buffer: &Vec<u8>) -> u16 {
    let msb:u16 = buffer[0].into();
    let lsb:u16 = buffer[1].into();
    u16::from_be(msb + lsb<<8)
}

pub fn from_bytes(buffer: Vec<u8>) -> Option<Mensagem> {
    let opcode:u16 = get_opcode(&buffer);
    match opcode {
        1 => Some(Mensagem::rrq(Requisicao::from_bytes(buffer))),
        2 => Some(Mensagem::wrq(Requisicao::from_bytes(buffer))),
        3 => Some(Mensagem::data(Data::from_bytes(buffer))),
        4 => Some(Mensagem::ack(Ack::from_bytes(buffer))),
        5 => Some(Mensagem::err(Err::from_bytes(buffer))),
        _ => None
    }
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
pub struct Data {
    block: u16,
    body: Vec<u8>
}

// Mensagem de confirmação
pub struct Ack {
    block: u16
}

// Mensagem de erro
pub struct Err {
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
    pub fn from_bytes(buffer: Vec<u8>) -> Self {
        let opcode:u16 = get_opcode(&buffer);
        let tipo = match opcode {
            1 => TipoReq::RRQ,
            2 => TipoReq::WRQ,
            _ => {
                panic!("opcode inválido para RRQ ou WRQ");
            }
        };
        let name = get_string(&buffer[2..]);
        let modo = get_string(&buffer[name.len()+3..]);
        Requisicao{
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
        }
    }
}