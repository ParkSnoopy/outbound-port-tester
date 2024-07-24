use ansi_escapes;
use termsize;
use ansi_term::{ Color };
use std::fmt::{ Debug };

use futures::{ stream, StreamExt };
use reqwest::{ Client };
use tokio;

use std::time::{ Duration, Instant };
use std::sync::{ Arc, Mutex };
use std::iter::{ repeat };

use clap::{ Parser };


#[derive(Parser, Debug)]
#[command(name = "Outbound Port Tester")]
#[command(version = "v0.1.2")]
#[command(author = "ParkSnoopy")]
#[command(
    about = "Local NAT outbound port restriction testing tool",
    long_about = "Test the ports from inside to outside of the network. \nA simple self-hosted all-port echo server may allow fast and reliable testing.\n"
)]
struct Args {
    // Protocol to use
    #[arg(long, default_value_t={"http".to_string()})]
    protocol: String,

    // Test server URL
    #[arg(long, default_value_t={"portquiz.net".to_string()})]
    host: String,

    // page path
    #[arg(long, default_value_t={"".to_string()})]
    path: String,

    // Concurrent Requests
    #[arg(long, short='N')]
    concurrent: usize,

    // Timeout
    #[arg(long, short='t', default_value_t=120)]
    timeout: u64,

    // Port Range Min
    #[arg(long, short='m', default_value_t=1)]
    fromport: u64,

    // Port Range Max
    #[arg(long, short='M', default_value_t=65535)]
    toport: u64,

    // Debug
    #[arg(long, short='d', action)]
    debug: bool,

    // List Closed
    #[arg(long, short='B', action)]
    list_blocked: bool,
}


#[tokio::main]
async fn main() {
    init();

    // Get User Arguments
    let args = Args::parse();

    let client = get_client(&args);
    let urls = get_urls(&args);

    let t0 = Instant::now();
    let total: usize = args.toport as usize - args.fromport as usize + 1;
    let done = Arc::new(Mutex::new(0_usize));

    print_progress(0, total);

    let request_results = stream::iter(urls)
        .map(|(port, url)| {
            let client = &client;
            async move {
                (port, client.get(url).send().await) // (u64, Future<Result<Response,Error>>)
            }
        })
        .buffer_unordered(args.concurrent);

    let opened_ports = Arc::new(Mutex::new( Vec::new() ));

    request_results
        .for_each(|(port, rr)| {
        let done_cloned = done.clone();
        let opened_ports_cloned = opened_ports.clone();
        async move {
            match rr {
                Ok (_r) => { opened_ports_cloned.lock().expect("Get Mutex Failed").push(port); },
                Err(_e) => { },
            }
            *done_cloned.lock().expect("Get Mutex Failed") += 1;
            print_progress(*done_cloned.lock().expect("Get Mutex Failed"), total);
            if args.debug { 
                println!("{}", Color::Yellow.paint(format!("[DEBUG] {:?}", 
                    opened_ports_cloned.lock().expect("Get Mutex Failed")
                )))
            }
        }})
        .await;


    // Result Statistics
    println!("\n\n\nTime elapsed: {:.0?}", t0.elapsed());
    println!();

    if let Ok(port_list) = Arc::try_unwrap(opened_ports) {
        let mut port_list = port_list.into_inner().unwrap();
        port_list.dedup();
        port_list.retain(|&v| v != 0);
        port_list.sort();

        let port_list = if args.list_blocked {
            let mut all_ports: Vec<u64> = (args.fromport..=args.toport).collect();
            all_ports.retain(|&v| !port_list.contains(&v) );
            all_ports
        } else {
            port_list
        };

        if port_list.len() > 0 {
            println!("List of ports {}",
                opened_or_closed(&args)
            );
            for port in port_list {
                println!("{}", Color::Fixed(69).paint(format!("  - {port:>5}")));
            }
        } else {
            let painted = format!("  - {} {} {} {} {}{}",
                Color::Fixed(9).paint("None of port between"),
                Color::Fixed(51).paint(format!("{}",args.fromport)),
                Color::Fixed(9).paint("to"),
                Color::Fixed(51).paint(format!("{}",args.toport)),
                Color::Fixed(9).paint(opened_or_closed(&args)),
                Color::Fixed(9).paint("..."),
            );
            println!("{}", painted);
        }

    } else {
        println!("  - Retrieving MutexGuard into Normal Vec Failed...");
    }

    cleanup();
}


fn init() {
    #[cfg(target_family = "windows")]
    {
        use ansi_term::enable_ansi_support;
        let _enabled = enable_ansi_support();
    }
    print!("{}\n\n{}",
        //ansi_escapes::CursorHide,
        ansi_escapes::ClearScreen,
        ansi_escapes::CursorSavePosition,
    );
}
fn cleanup() {
    print!("\n\n", 
        //ansi_escapes::CursorShow
    );
}


fn get_client(args: &Args) -> Client {
    Client::builder()
        .timeout(Duration::from_secs(args.timeout))
        .build()
        .unwrap()
}

fn get_urls(args: &Args) -> Vec<(u64, String)> {
    (args.fromport..=args.toport)
        .map(|port| (port, format!( "{}://{}:{}/{}", args.protocol, args.host, port, args.path )) )
        .collect()
}

fn print_progress(done:usize, total:usize) {
    let ansi_escape_init = format!("{}",
        ansi_escapes::CursorRestorePosition,
    );
    let prebuilt = Color::Cyan.paint(
        format!("{} ({}/{})", ansi_escape_init, done, total)
    );
    println!("{} [ {} ]", 
        prebuilt, 
        Color::Fixed(118).paint( progress_to_bar(done, total, prebuilt.len()) ),
    );
}

fn progress_to_bar(done:usize, total:usize, used_space:usize) -> String {
    let progress_width = termsize::get().unwrap().cols as usize - used_space -3;
    let fill: String = repeat("#")
        .take( progress_width*done/total)
        .collect();
    let void: String = repeat(" ")
        .take( progress_width - fill.len() )
        .collect();
    format!("{}{}", fill, void)
}

fn opened_or_closed(args: &Args) -> &str {
    if args.list_blocked {
        "closed"
    } else {
        "opened"
    }
}
