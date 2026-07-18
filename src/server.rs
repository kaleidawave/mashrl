#[allow(non_snake_case)]
pub mod mimetypes;

use super::http;

use std::{
	borrow::Cow,
	io::{BufRead, BufReader, Read, Write},
	net::TcpListener,
};

pub use native_tls::Identity;

/// Useful for general server returns
pub type BoxedDynRead = Box<dyn std::io::Read + Send>;

pub struct LocalHost {
	port: u16,
}

impl std::net::ToSocketAddrs for LocalHost {
	type Iter = std::vec::IntoIter<std::net::SocketAddr>;

	fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
		format!("127.0.0.1:{port}", port = self.port).to_socket_addrs()
	}
}

#[cfg(windows)]
pub fn get_local_ip_config(port: u16) -> std::net::SocketAddrV4 {
	let out =
		std::command::Command::new("ipconfig").output().expect("could not spawn 'ipconfig'").stdout;

	let out = String::from_utf8(out).unwrap();
	out.lines().find(|line| {
		if line.trim_start().starts_with("IPv4 Address") {
			let value = line.rsplit_once(':').unwrap().1.trim();
			Some(std::net::SocketAddrV4::new(value.parse().unwrap(), port))
		} else {
			None
		}
	})
}

#[cfg(unix)]
pub fn get_local_ip_config(port: u16) -> std::net::SocketAddrV4 {
	let out = std::process::Command::new("ipconfig").arg("getifaddr").arg("en0").output().unwrap();
	let value = String::from_utf8(out.stdout).unwrap();
	std::net::SocketAddrV4::new(value.parse().unwrap(), port)
}

pub type Body<'a, S> = http::Body<std::io::Take<BufReader<&'a mut S>>>;

pub fn parse_request<'a, S>(
	stream: &'a mut S,
	first_line: &'a mut String,
) -> http::Request<'a, Body<'a, S>>
where
	S: Read + Send + 'a,
{
	let mut reader = BufReader::new(stream);
	let _out = reader.read_line(first_line);
	let mut headers_buf = String::new();

	// TODO direct TLS handshake -> non TLS handlshake
	// #[cfg(not(feature = "https"))]
	// let Some((method, path)) = first_line.split_once(' ') else {
	//     let mut seek: [u8; 1] = [0; 1];
	//     let _ = std::io::Read::read_exact(&mut reader, &mut seek);
	//     if let 0x16 = seek[0] {
	//         let error_response = "HTTP/1.1 426 Upgrade Required\r\n\
	//              Content-Type: text/plain\r\n\
	//              Content-Length: 51\r\n\
	//              Connection: close\r\n\
	//              \r\n\
	//              This server does not support TLS. Use HTTP instead.";

	//         let _ = stream.write_all(error_response.as_bytes());
	//     }
	//     dbg!(&seek);
	//     continue;
	// };

	// parse response
	let (left, right) = first_line.split_once(' ').unwrap();
	let method = left;
	let (left, _protocol) = right.split_once(' ').unwrap();
	let path = left;

	// Not sure what the default value should be here
	let mut content_length: u64 = 0;

	loop {
		let Ok(bytes_read) = reader.read_line(&mut headers_buf) else {
			eprintln!("no header");
			continue;
		};

		let last = headers_buf.len() - bytes_read;
		let line = &headers_buf[last..].trim_end();

		// end of headers
		if line.is_empty() {
			let _ = headers_buf.truncate(headers_buf.len() - 2);
			break;
		}

		if let Some((key, value)) = line.split_once(':') {
			if key.eq_ignore_ascii_case("Content-Length") {
				let value = value.trim();
				if let Ok(value) = <u64 as std::str::FromStr>::from_str(value) {
					content_length = value;
				}
			} else if key.eq_ignore_ascii_case("Transfer-Encoding") {
				// Chunked responses should not be limited
				if value.contains("chunked") {
					content_length = u64::MAX;
				}
			}
		}
	}

	let reader = std::io::Read::take(reader, content_length);

	http::Request {
		method: http::Method(Cow::Borrowed(method)),
		path: Cow::Borrowed(path),
		headers: http::Headers::from_string(headers_buf),
		body: http::Body::new(reader),
	}
}

/// creates a (single threaded) HTTPS server
// #[cfg(feature = "https")]
// pub fn open_https_server<RB: std::io::Read>(
//     port: impl std::net::ToSocketAddrs,
//     identity: native_tls::Identity,
//     callback: impl for<'a> Fn(http::Request<'a, RB>) -> http::Response<'static, Body>,
// ) {
//     let listener = TcpListener::bind(port).expect("could not open listener on port");

//     // #[cfg(feature = "https")]
//     let acceptor = native_tls::TlsAcceptor::new(identity).unwrap();

//     for stream in listener.incoming().flatten() {
//         // TODO http blocks here
//         let mut stream = acceptor
//             .accept(stream)
//             .expect("could not accept TLS stream");

//         let mut first_line = String::new();

//         let request = parse_request(&mut stream, &mut first_line);
//         let mut response = callback(request);

//         {
//             // TODO http2 and http3?
//             stream.write_all(b"HTTPS/1.1 ").unwrap();
//             // TODO custom codes
//             let code = response.code.to_str();
//             stream.write_all(code.as_bytes()).unwrap();
//             stream.write_all(b"\r\n").unwrap();

//             debug_assert!(
//                 response.headers.is_valid(),
//                 "Invalid headers {headers:?}",
//                 headers = &response.headers.0
//             );
//             // TODO trim end
//             stream
//                 .write_all(response.headers.0.trim_end().as_bytes())
//                 .unwrap();
//             stream.write_all(b"\r\n").unwrap();
//             stream.write_all(b"\r\n").unwrap();

//             std::io::copy(&mut response.body, &mut stream).unwrap();
//         }
//     }
// }

pub type TLS = native_tls::TlsStream<std::net::TcpStream>;
pub type TCP = std::net::TcpStream;

/// creates a (single threaded) HTTP server
pub fn open_http_server<RB: std::io::Read>(
	port: impl std::net::ToSocketAddrs,
	mut callback: impl for<'a> FnMut(http::Request<'a, Body<'a, TCP>>) -> http::Response<'static, RB>,
) {
	let listener = TcpListener::bind(port).expect("could not open listener on port");

	for mut stream in listener.incoming().flatten() {
		let mut first_line = String::new();

		let request = parse_request(&mut stream, &mut first_line);
		let mut response = callback(request);

		{
			// TODO http2 and http3?
			stream.write_all(b"HTTP/1.1 ").unwrap();
			// TODO custom codes
			let code = response.code.to_str();
			stream.write_all(code.as_bytes()).unwrap();
			stream.write_all(b"\r\n").unwrap();

			debug_assert!(
				response.headers.is_valid(),
				"Invalid headers {headers:?}",
				headers = &response.headers.0
			);
			// TODO trim end
			stream.write_all(response.headers.0.trim_end().as_bytes()).unwrap();
			stream.write_all(b"\r\n").unwrap();
			stream.write_all(b"\r\n").unwrap();

			std::io::copy(&mut response.body, &mut stream).unwrap();
		}
	}
}
