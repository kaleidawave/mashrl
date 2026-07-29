#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]

use native_tls::{TlsConnector, TlsStream};
use std::io::Take;
use std::net::TcpStream;

pub mod http;

#[cfg(feature = "extras")]
pub mod extras;
#[cfg(feature = "server")]
pub mod server;

/// Not the HTTP body (for some reason)
fn write_method_path_and_headers<T: std::io::Read, S: std::io::Write>(
	request: &http::Request<'_, T>,
	mut stream: S,
) -> Result<S, Box<dyn std::error::Error>> {
	let http::Request { method, path, headers, body: _ } = request;

	let method: &str = &method.0;

	let base = format!("{method} /{path} HTTP/1.1\r\n");

	stream.write_all(base.as_bytes())?;
	debug_assert!(headers.is_valid(), "Invalid headers {headers:?}", headers = &headers.0);
	stream.write_all(headers.0.trim_end().as_bytes())?;
	stream.write_all(b"\r\n")?;
	stream.write_all(b"\r\n")?;

	Ok(stream)
}

/// Start the http request, sending [`http::Headers<'_>`]
fn initiate_stream_tls<T: std::io::Read>(
	request: &http::Request<'_, T>,
) -> Result<TlsStream<TcpStream>, Box<dyn std::error::Error>> {
	let host = request.headers.get("host").unwrap();

	let (root, url) = if let Some((root, _port)) = host.rsplit_once(':') {
		(root, host.to_owned())
	} else {
		// https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Host
		// "If no port is included, the default port for the service requested is
		// implied (e.g., 443 for an HTTPS URL, and 80 for an HTTP URL)."
		let port = 443;
		let root = host;
		(root, format!("{root}:{port}"))
	};
	let tcp_stream = TcpStream::connect(url)?;
	let connector = TlsConnector::new()?;
	let tls_stream = connector.connect(root, tcp_stream)?;
	write_method_path_and_headers(request, tls_stream)
}

// fn initiate_stream_non_tls<T: std::io::Read>(
//     request: &http::Request<'_, T>,
// ) -> Result<TcpStream, Box<dyn std::error::Error>> {
//     let url = format!("{root}:80", root = request.root);
//     let tcp_stream = TcpStream::connect(url)?;
//     write_method_path_and_headers(&request, tcp_stream)
// }

/// # Errors
///
/// returns an error if the returned http response is invalid
pub fn make_get_request(
	root: &str,
	path: &str,
	mut headers: http::Headers<'_>,
) -> Result<Resp<TlsStream<TcpStream>>, Box<dyn std::error::Error>> {
	// TODO http
	let host = root.strip_prefix("https://").unwrap_or(root);
	headers.append("Host", host);
	// TODO
	// headers.append("Connection", "close");
	let request = http::Request::new_get(path, headers);
	make_request(request)
}

// pub type Resp<S> = http::Response<'static, http::Body<Take<S>>>;
pub type Resp<S> = http::Response<'static, http::Body<Take<std::io::BufReader<S>>>>;

/// TODO request like. `(&str, &str)` etc.
/// # Errors
///
/// returns an error if the returned http response is invalid
pub fn make_request<T: std::io::Read + Send>(
	mut request: http::Request<'_, T>,
) -> Result<Resp<TlsStream<TcpStream>>, Box<dyn std::error::Error>> {
	let mut stream = initiate_stream_tls(&request)?;
	let _out = std::io::copy(&mut request.body, &mut stream)?;
	parse_http_response(stream)

	// TODO monomorphism will 2x this...
	// if let Ok(ref response) = response && response.code == http::ResponseCode::UPGRADE_REQUIRED {
	//     // try non TLS
	//     let mut stream = initiate_stream(&request)?;
	//     let _out = std::io::copy(&mut request.content, &mut stream)?;
	//     parse_http_response(&mut stream)
	// } else {
	// response
	// }
}

pub(crate) fn parse_http_response<B: std::io::Read>(
	stream: B,
) -> Result<Resp<B>, Box<dyn std::error::Error + 'static>> {
	use std::io::{BufRead, BufReader};

	let mut reader = BufReader::new(stream);

	let code: http::ResponseCode = {
		let mut line = String::new();
		let Ok(_bytes_read) = reader.read_line(&mut line) else {
			return Err("no code".into());
		};

		let code = line
			.trim_end()
			.split_once(' ')
			.and_then(|(_method, item)| http::ResponseCode::from_line(item).ok());

		let Some(code) = code else {
			return Err(format!("invalid response code: {line:?}").into());
		};
		code
	};

	let mut headers_buf = String::new();
	let mut content_length: u64 = 0;

	loop {
		let Ok(bytes_read) = reader.read_line(&mut headers_buf) else {
			return Err("no code".into());
		};

		let last = headers_buf.len() - bytes_read;
		let line = &headers_buf[last..].trim_end();

		if let Some((key, value)) = line.split_once(':') {
			if key.eq_ignore_ascii_case("Content-Length") {
				let value = value.trim();
				if let Ok(value) = str::parse::<u64>(value) {
					content_length = value;
				}
			} else if key.eq_ignore_ascii_case("Transfer-Encoding") && value.contains("chunked") {
				content_length = u64::MAX;
			}
		}

		if line.is_empty() {
			// finished headers
			// drop last '\r\n'
			let _ = headers_buf.drain(headers_buf.len() - 2..);
			break;
		}
	}

	let reader = std::io::Read::take(reader, content_length);

	let headers = http::Headers::from_string(headers_buf);
	assert!(headers.is_valid());

	let body = http::Body(reader);

	let response = http::Response { code, headers, body };

	Ok(response)
}
