use tftp::{ClienteTFTP,Status};
use clap::{Parser, Subcommand};

/// Um pequeno cliente TFTP experimental
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
   #[command(subcommand)]
   command: Commands,

   /// Name of the person to greet
   #[arg(short, long)]
   server: String,

   /// Number of times to greet
   #[arg(short, long, default_value_t = 69)]
   port: u16,

   /// Max retries when sending a file
   #[arg(short, long, default_value_t = 3)]
   retries: u16,

   /// Timeout waiting from server
   #[arg(short, long, default_value_t = 5)]
   timeout: u16

}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Envia um arquivo
    #[command(arg_required_else_help = true)]
    Envia {
        /// The local file to send
        local: String,

        /// The name of the remote file
        #[arg(default_value_t = String::new())]
        remote: String
    },
    /// Recebe um arquivo
    #[command(arg_required_else_help = true)]
    Recebe {
        /// The name of the remote file
        remote: String,

        /// The local file to send
        #[arg(default_value_t = String::new())]
        local: String,
    }
}

fn print_status(status: Status) {
   match status {
      Status::OK => println!("Arquivo recebido e gravado"),
      Status::Error(e) => println!("Erro: {:?}", e),
      Status::Unknown => println!("Erro desconhecido"),
      Status::Timeout => println!("Timeout"),
      Status::MaxRetriesExceeded => println!("retransmissões excedidas")
   }
}   

fn main() {
   let args = Cli::parse();
   let cliente = ClienteTFTP::new(&args.server, args.port);

   match args.command {
      Commands::Envia{local: name, remote: rname} => {
         println!("envia: {} -> {}", name, rname);
         let rname = match name.as_str() {
            "" => &name,
            _ => &rname
         };
         print_status(cliente.envia(&name, rname));
      },
      Commands::Recebe{remote: rname, local: name} => {         
         println!("recebe: {} -> {}", rname, name);
         let name = match name.as_str() {
            "" => &rname,
            _ => &name
         };
         print_status(cliente.recebe(&rname, name));
      }
   };
}