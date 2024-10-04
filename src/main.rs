use clap::{Parser, Subcommand};
use tftp::{ClienteTFTP,Status};
use std::{net::Ipv4Addr, process::exit};

/// Um pequeno cliente TFTP experimental
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
   /// Nome ou endereço IP do servidor
   #[arg(short, long)]
   server: String,

   /// Port do servidor (default: 69)
   #[arg(short, long, default_value_t = 69)]
   port: u16,

   /// comando: envia ou recebe
   #[command(subcommand)]
   cmd: Comandos,
}

#[derive(Debug)]
#[derive(Subcommand)]
enum Comandos {
   envia {arquivo: String},
   recebe {arquivo: String} 
}

fn show_status(status: Status) -> String {
   match status {
      Status::OK => "Arquivo recebido e gravado".to_owned(),
      Status::Error(e) => format!("Erro: {:?}", e),
      Status::Unknown =>  "Erro desconhecido".to_owned(),
      Status::Timeout =>  "Timeout".to_owned(),
      Status::MaxRetriesExceeded =>  "retransmissões excedidas".to_owned()
   }
}

fn main() {
   let args = Args::parse();

   let cliente = ClienteTFTP::new(&args.server, args.port);
   let status;

   match &args.cmd {
      Comandos::envia{arquivo} => {
         println!("enviando {} para {}", arquivo, &args.server);
         status = cliente.envia(arquivo);
      }
      Comandos::recebe {arquivo} => {
         println!("recebendo {} de {}", arquivo, &args.server);
         status = cliente.recebe(arquivo, arquivo);
      }
   }

   println!("{}", show_status(status));

   std::process::exit(0);
}
