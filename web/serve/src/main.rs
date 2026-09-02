use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::Path,
};

fn main() -> std::io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Serving {} at http://127.0.0.1:8080", root.display());
    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut lines = BufReader::new(&stream).lines();
        let request = lines.next().transpose()?.unwrap_or_default();
        for line in lines.by_ref() {
            if line?.is_empty() {
                break;
            }
        }
        let path = request.split(' ').nth(1).unwrap_or("/").trim_start_matches('/');
        let path = if path.is_empty() { "index.html" } else { path };
        let mime = match Path::new(path).extension().and_then(|ext| ext.to_str()) {
            Some("html") => "text/html",
            Some("js") => "text/javascript",
            Some("wasm") => "application/wasm",
            Some("png") => "image/png",
            _ => "application/octet-stream",
        };
        match fs::read(root.join(path)) {
            Ok(body) => {
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )?;
                stream.write_all(&body)?;
            }
            Err(_) => stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")?,
        }
    }
    Ok(())
}
