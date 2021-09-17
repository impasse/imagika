use structopt::StructOpt;

use imagika::compress_pptx;

#[derive(Debug, StructOpt)]
#[structopt(name = "imagika", about = "A tool for compress pptx")]
struct Opts {
    #[structopt(short, long)]
    input: String,
    #[structopt(short, long)]
    output: String,
}


fn main() {
    let opts: Opts = Opts::from_args();

    println!("Thread pool size: {}", rayon::current_num_threads());

    match compress_pptx(opts.input, opts.output) {
        Ok(_) => {
            println!("Finished");
        }
        Err(e) => {
            println!("Failed:\n{}", e);
        }
    }
}
