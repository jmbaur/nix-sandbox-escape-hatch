use log::debug;
use std::io::{prelude::*, BufReader, BufWriter};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug)]
enum Error {
    BadRuntimeDirectory,
    CommandFailedStatusCode(u8),
    CommandRunFailed,
    InvalidArgs,
    Io(std::io::Error),
    MissingRuntimeDirectory,
    MissingSocket,
    MissingSubcommand,
    NoArgs,
    TooManySockets,
    UnknownServerLocation,
    UnknownSubcommand,
    InvalidBuilderExe,
    MissingBuilderExe,
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

enum Subcommand {
    Client,
    Server,
}

impl FromStr for Subcommand {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "client" => Ok(Self::Client),
            "server" => Ok(Self::Server),
            _ => Err(Error::UnknownSubcommand),
        }
    }
}

fn clean_dir(dir: &PathBuf) -> Result<(), Error> {
    for child in std::fs::read_dir(dir)? {
        let child = child?;
        if child.file_type()?.is_dir() {
            clean_dir(&child.path())?;
        } else {
            std::fs::remove_file(&child.path())?;
        }
    }

    Ok(())
}

fn copy_dir_recursively(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), Error> {
    std::fs::create_dir_all(&dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursively(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }

    Ok(())
}

fn run_server(command: Vec<String>) -> Result<(), Error> {
    env_logger::init();

    if command.len() > 1 {
        return Err(Error::InvalidBuilderExe);
    }

    let exe = command.first().ok_or(Error::MissingBuilderExe)?;

    let runtime_directory =
        PathBuf::from(std::env::var("RUNTIME_DIRECTORY").map_err(|e| match e {
            std::env::VarError::NotPresent => Error::MissingRuntimeDirectory,
            std::env::VarError::NotUnicode(_) => Error::BadRuntimeDirectory,
        })?);

    clean_dir(&runtime_directory)?;

    let command_out_path = runtime_directory.join("out");

    let mut listen_fds = systemd::daemon::listen_fds(true)?.iter();

    let listen_fd = listen_fds.next().ok_or(Error::MissingSocket)?;
    if listen_fds.next().is_some() {
        return Err(Error::TooManySockets);
    }

    let mut stream = BufReader::new(unsafe { std::net::TcpStream::from_raw_fd(listen_fd) });

    debug!("reading args from client");

    let mut args = Vec::new();
    let bytes_read = stream.read_until(b'\0', &mut args)?;
    args.truncate(bytes_read - 1); // exclude null byte
    let client_args = String::from_utf8(args).or(Err(Error::InvalidArgs))?;

    let mut builder_cmd = std::process::Command::new(exe);
    builder_cmd.args(client_args.split(' '));
    builder_cmd.env("out", command_out_path);

    debug!("running {:?}", builder_cmd);

    match builder_cmd.output() {
        Ok(output) => {
            std::fs::write(runtime_directory.join("stdout"), output.stdout)?;
            std::fs::write(runtime_directory.join("stderr"), output.stderr)?;
            std::fs::write(
                runtime_directory.join("status_code"),
                output
                    .status
                    .code()
                    .unwrap_or_default()
                    .to_string()
                    .as_bytes(),
            )?;
        }
        Err(e) => {
            std::fs::write(runtime_directory.join("error"), e.to_string())?;
        }
    }

    debug!("writing archive");

    let mut archive = tar::Builder::new(BufWriter::new(stream.into_inner()));
    archive.append_dir_all("result", runtime_directory)?;
    archive.follow_symlinks(false);
    archive.finish()?;

    Ok(())
}

fn handle_client_stream<S>(args: Vec<String>, mut stream: S) -> Result<(), Error>
where
    S: Read + Write,
{
    let args = args.join(" ");

    stream.write_all(args.as_bytes())?;
    stream.write(&[b'\0'])?;
    stream.flush()?;

    let mut archive = tar::Archive::new(BufReader::new(stream));

    let unpack_dir = PathBuf::from("tmp");
    std::fs::create_dir_all(&unpack_dir)?;
    clean_dir(&unpack_dir)?;

    archive.unpack(&unpack_dir)?;

    summarize_results(&unpack_dir)?;
    unpack_to_derivation(&unpack_dir)?;

    Ok(())
}

fn unpack_to_derivation(unpack_dir: &PathBuf) -> Result<(), Error> {
    let out_result_path = unpack_dir.join("result/out");

    if let Ok(out) = std::env::var("out") {
        let nix_dollar_out = PathBuf::from(out);

        if let Ok(md) = std::fs::metadata(&out_result_path) {
            if md.is_dir() {
                copy_dir_recursively(out_result_path, nix_dollar_out)?;
            } else if md.is_file() {
                std::fs::copy(out_result_path, nix_dollar_out)?;
            }
        }
    }

    Ok(())
}

fn summarize_results(unpack_dir: &PathBuf) -> Result<(), Error> {
    match std::fs::read_to_string(unpack_dir.join("result/error")) {
        Ok(error_contents) => {
            eprintln!("FAILED TO RUN ESCAPE HATCH COMMAND:\n{}", error_contents);
            return Err(Error::CommandRunFailed);
        }
        Err(_) => {}
    }

    match std::fs::read_to_string(unpack_dir.join("result/stdout")) {
        Ok(stdout) => {
            if !stdout.is_empty() {
                eprintln!("STDOUT:\n{}", stdout);
            }
        }
        Err(_) => {}
    }

    match std::fs::read_to_string(unpack_dir.join("result/stderr")) {
        Ok(stderr) => {
            if !stderr.is_empty() {
                eprintln!("STDERR:\n{}", stderr);
            }
        }
        Err(_) => {}
    }

    match std::fs::read_to_string(unpack_dir.join("result/status_code"))
        .map(|code| u8::from_str_radix(&code, 10))
    {
        Ok(Ok(0)) => {}
        Ok(Ok(code)) => {
            eprintln!("ESCAPE HATCH COMMAND EXITED WITH STATUS {}", code);
            return Err(Error::CommandFailedStatusCode(code));
        }
        _ => {}
    }

    Ok(())
}

fn run_client(args: Vec<String>) -> Result<(), Error> {
    if args.is_empty() {
        return Err(Error::NoArgs);
    }

    // systemd-socket-activate is a useful tool for development of socket-activated services,
    // however the tool does not support unix sockets, so we instead allow for use of a Unix socket
    // path or IP address depending on which environment variables are set.
    match (
        std::env::var("NIX_SANDBOX_ESCAPE_HATCH_PATH"),
        std::env::var("SYSTEMD_SOCKET_ACTIVATE_ADDR"),
    ) {
        // prioritize socket path
        (Ok(socket_path), Err(_)) | (Ok(socket_path), Ok(_)) => {
            handle_client_stream(args, std::os::unix::net::UnixStream::connect(socket_path)?)
        }
        (Err(_), Ok(ip_addr)) => handle_client_stream(args, std::net::TcpStream::connect(ip_addr)?),
        (Err(_), Err(_)) => Err(Error::UnknownServerLocation),
    }
}

fn main() -> Result<(), Error> {
    let mut args = std::env::args().into_iter().skip(1);

    let subcommand = args
        .next()
        .map(|s| Subcommand::from_str(s.as_str()))
        .transpose()?
        .ok_or(Error::MissingSubcommand)?;

    match subcommand {
        Subcommand::Client => run_client(args.collect()),
        Subcommand::Server => run_server(args.collect()),
    }
}
