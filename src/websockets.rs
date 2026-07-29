use super::http;
use std::net::{TcpListener, ToSocketAddrs};

#[repr(u8)]
enum OpCode {
	// Continuation = 0,
	Text = 1,
	Binary = 2,
	Close = 8,
	// Ping = 9,
	// Pong = 10,
}

#[derive(Debug)]
pub enum Message {
	Text(String),
	Binary(Vec<u8>),
	Close,
}

pub struct Client<T: std::io::Write>(pub(crate) T);

impl Client<std::net::TcpStream> {
	// Client
	pub fn connect(url: &str, key: &[u8; 16]) -> Result<Self, Box<dyn std::error::Error>> {
		if let Some(after) = url.strip_prefix("ws://") {
			use std::net::TcpStream;
			// use native_tls::TlsConnector;
			use std::io::Write;

			let (host, path) = after.split_once('/').unwrap_or((after, ""));

			// let (_root, url) = if let Some((root, _port)) = host.rsplit_once(':') {
			// 	(root, host.to_owned())
			// } else {
			// 	// https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Host
			// 	// "If no port is included, the default port for the service requested is
			// 	// implied (e.g., 443 for an HTTPS URL, and 80 for an HTTP URL)."
			// 	let port = 443;
			// 	let root = host;
			// 	(root, format!("{root}:{port}"))
			// };

			let tcp_stream = TcpStream::connect(host)?;
			// let connector = TlsConnector::new()?;
			// let tls_stream = connector.connect(root, tcp_stream)?;

			let mut stream = tcp_stream;

			let key = base64(key);

			{
				// TODO should use header/Response abstraction
				write!(&mut stream, "GET /{path} HTTP/1.1\r\n")?;
				write!(&mut stream, "Host: {host}\r\n")?;
				write!(&mut stream, "Upgrade: websocket\r\n")?;
				write!(&mut stream, "Connection: upgrade\r\n")?;
				write!(&mut stream, "Sec-WebSocket-Key: {key}\r\n")?;
				write!(&mut stream, "Sec-WebSocket-Version: 13\r\n")?;
				stream.write_all(b"\r\n")?;

				let response = crate::parse_http_response(&mut stream)?;

				// dbg!(response.code, response.headers);
				assert_eq!(response.code, crate::http::ResponseCode::SWITCHING_PROTOCOLS);
				let accept = get_accept(&key);
				assert_eq!(response.headers.get("Sec-WebSocket-Accept"), Some(accept.as_str()));

				let stream = response.body.raw().into_inner();
				assert_eq!(stream.buffer(), &[]);
			}

			Ok(Client(stream))
		} else {
			panic!("invalid {url:?}");
		}
	}
}

impl<T: std::io::Write> Client<T> {
	pub fn write(&mut self, message: Message) -> std::io::Result<()> {
		write_message(&mut self.0, message)
	}
}

impl<T: std::io::Write + std::io::Read> Client<T> {
	pub fn read(&mut self) -> std::io::Result<Message> {
		read_message(&mut self.0)
	}
}

// WIP
impl<T: std::io::Write> Drop for Client<T> {
	fn drop(&mut self) {
		// TODO cannot unwrap
		let _ = self.write(Message::Close);
	}
}

fn write_message(stream: &mut impl std::io::Write, message: Message) -> std::io::Result<()> {
	fn write_length(
		stream: &mut impl std::io::Write,
		masked: bool,
		len: usize,
	) -> Result<(), std::io::Error> {
		let byte = masked.then_some(0b1000_0000).unwrap_or_default();
		match len {
			0..=125 => {
				stream.write(&[byte | len as u8])?;
			}
			// u16::MAX as usize
			126..=65_535 => {
				stream.write(&[byte | 126 as u8])?;
				stream.write(&u16::to_be_bytes(len as u16))?;
			}
			_ => {
				stream.write(&[byte | 127 as u8])?;
				stream.write(&u64::to_be_bytes(len as u64))?;
			}
		}
		Ok(())
	}

	match message {
		Message::Text(text) => {
			stream.write(&[0b1000_0000 | OpCode::Text as u8])?;
			// No mask
			write_length(stream, false, text.len())?;
			stream.write(text.as_bytes())?;
		}
		Message::Binary(content) => {
			stream.write(&[0b1000_0000 | OpCode::Binary as u8])?;
			// No mask
			write_length(stream, false, content.len())?;
			stream.write(&content)?;
		}
		Message::Close => {
			// Could give status here
			stream.write(&[0b1000_0000 | OpCode::Close as u8, 0])?;
		}
	}
	Ok(())
}

fn read_message(stream: &mut impl std::io::Read) -> std::io::Result<Message> {
	let (first, second) = {
		let mut buffer = [0u8; 2];
		stream.read_exact(&mut buffer)?;
		(buffer[0], buffer[1])
	};

	let _fin = first & 0b1000_0000 != 0;
	let _rsv1 = first & 0b0100_0000;
	let _rsv2 = first & 0b0010_0000;
	let _rsv3 = first & 0b0001_0000;
	let opcode = first & 0b1111;

	let masked = second & 0b1000_0000 != 0;

	// mut
	let mut payload_length: u64 = (second & 0b0111_1111).into();

	if payload_length == 126 {
		let mut buffer = [0u8; 2];
		stream.read_exact(&mut buffer)?;
		payload_length = u16::from_be_bytes(buffer) as u64;
	} else if payload_length == 127 {
		let mut buffer = [0u8; 8];
		stream.read_exact(&mut buffer)?;
		// TODO msb = 0?
		payload_length = u64::from_be_bytes(buffer);
	}

	let mask: [u8; 4] = if masked {
		let mut buffer = [0u8; 4];
		stream.read_exact(&mut buffer)?;
		buffer
	} else {
		[0; 4]
	};

	match opcode {
		// OpCode::Text => {
		1 => {
			let mut payload = vec![0u8; payload_length as usize];
			stream.read_exact(&mut payload)?;
			for i in 0..payload.len() {
				payload[i] ^= mask[i % 4];
			}
			// TODO `String::from_utf8` ?
			Ok(Message::Text(String::from_utf8_lossy(&payload).into_owned()))
		}
		// OpCode::Binary => {
		2 => {
			let mut payload = vec![0u8; payload_length as usize];
			stream.read_exact(&mut payload)?;
			for i in 0..payload.len() {
				payload[i] ^= mask[i % 4];
			}
			Ok(Message::Binary(payload))
		}
		// OpCode::Close => Ok(Message::Close),
		8 => Ok(Message::Close),
		_ => {
			panic!("Unknown opcode {opcode:?}");
		}
	}
}

#[cfg(feature = "server")]
pub fn open_http_and_websocket_server<RB: std::io::Read>(
	port: impl ToSocketAddrs,
	mut callback: impl for<'a> FnMut(
		http::Request<'a, crate::server::Body<'a, crate::server::TCP>>,
	) -> http::Response<'static, RB>,
	mut websocket_callback: impl FnMut(Client<std::net::TcpStream>),
) -> Result<(), Box<dyn std::error::Error>> {
	use std::io::Write;

	let listener = TcpListener::bind(port)?;

	for mut stream in listener.incoming().flatten() {
		let request = crate::server::parse_request(&mut stream);

		if let Some("websocket") = request.headers.get("Upgrade") {
			let key = request.headers.get("Sec-WebSocket-Key").unwrap().to_owned();
			drop(request);

			stream.write_all(b"HTTP/1.1 101 Switching Protocols\r\n")?;
			stream.write_all(b"Upgrade: websocket\r\n")?;
			stream.write_all(b"Connection: Upgrade\r\n")?;

			{
				let accept = get_accept(&key);
				stream.write_all(b"Sec-WebSocket-Accept: ")?;
				stream.write_all(accept.as_bytes())?;
				stream.write_all(b"\r\n")?;
			}
			stream.write_all(b"\r\n")?;
			stream.flush()?;

			websocket_callback(Client(stream));
		} else {
			let mut response = callback(request);

			{
				// TODO http2 and http3?
				stream.write_all(b"HTTP/1.1 ")?;
				// TODO custom codes
				let code = response.code.to_str();
				stream.write_all(code.as_bytes())?;
				stream.write_all(b"\r\n")?;

				debug_assert!(
					response.headers.is_valid(),
					"Invalid headers {headers:?}",
					headers = &response.headers.0
				);
				// TODO trim end
				stream.write_all(response.headers.0.trim_end().as_bytes())?;
				stream.write_all(b"\r\n")?;
				stream.write_all(b"\r\n")?;

				std::io::copy(&mut response.body, &mut stream)?;
			}
		}
	}

	Ok(())
}

/// ```rust
/// use mashrl::websockets::get_accept;
/// let (key, accept) = ("dGhlIHNhbXBsZSBub25jZQ==", "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
/// assert_eq!(get_accept(key), accept);
/// ```
pub fn get_accept(key: &str) -> String {
	const MAGIC: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

	let concat = format!("{key}{MAGIC}");
	let hash = sha1(concat.as_bytes());
	let accept = base64(hash.as_slice());
	accept
}

/// ```rust
/// use mashrl::websockets::base64;
/// assert_eq!(base64(b"Many hands make light work."), "TWFueSBoYW5kcyBtYWtlIGxpZ2h0IHdvcmsu");
/// assert_eq!(base64(b""), "");
/// assert_eq!(base64(b"M"), "TQ==");
/// assert_eq!(base64(b"Ma"), "TWE=");
/// assert_eq!(base64(b"Man"), "TWFu");
/// assert_eq!(base64(b"Many"), "TWFueQ==");
/// ```
pub fn base64(input: &[u8]) -> String {
	static ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

	let mut buf = String::new();
	let mut acc: u8 = 0;
	// let to = input.len() + (3 - input.len() % 3);
	for (idx, byte) in input.iter().enumerate() {
		// let byte = input.as_bytes().get(idx).copied().unwrap_or_default();
		let offset = idx % 3;
		// AAAAAAAABBBBBBBBCCCCCCCC idx
		// aaaaaabbbbbbccccccdddddd
		match offset {
			0 => {
				let key = byte & 0b1111_1100;
				buf.push(ALPHABET[(key >> 2) as usize] as char);
				acc = byte << 6;
			}
			1 => {
				let key = acc | (byte & 0b1111_0000) >> 2;
				buf.push(ALPHABET[(key >> 2) as usize] as char);
				acc = byte << 4;
			}
			2 => {
				let key = acc | (byte & 0b1100_0000) >> 4;
				buf.push(ALPHABET[(key >> 2) as usize] as char);
				acc = byte << 2;

				// let key = acc | (byte & 0b0000_0000) >> 8; implies key = acc;
				buf.push(ALPHABET[(acc >> 2) as usize] as char);
				acc = 0;
			}
			_ => unreachable!(),
		}
	}

	if buf.len() % 4 != 0 {
		buf.push(ALPHABET[(acc >> 2) as usize] as char);
	}

	let padding = match buf.len() % 4 {
		2 => "==",
		3 => "=",
		_ => "",
	};
	buf.push_str(padding);

	buf
}

// TODO Could use `Wrapping` but `rotate_left` unstable + fine for now
// 160-bit number
pub fn sha1(input: &[u8]) -> [u8; 20] {
	let mut h0: u32 = 0x67452301;
	let mut h1: u32 = 0xEFCDAB89;
	let mut h2: u32 = 0x98BADCFE;
	let mut h3: u32 = 0x10325476;
	let mut h4: u32 = 0xC3D2E1F0;

	// let mut input = input.to_vec();
	// {
	// 	let ml: u64 = input.len() as u64 * 8;
	// 	input.push(0b1000_0000u8);
	// 	// this is the problem
	// 	while input.len() % 64 != 56 {
	// 		input.push(0);
	// 	}
	// 	input.extend_from_slice(&ml.to_be_bytes());
	// 	assert_eq!(input.len() % 64, 0, "{m} {l}", m=input.len() % 64, l=input.len());
	// }
	// let total_length = input.len();

	let mut total_length = input.len() + 1;
	if total_length % 64 > 56 {
		total_length += 64;
	}

	let mut i = 0;
	while i < total_length {
		let mut chunk: [u32; 80] = [0u32; 80];

		// let iter = input[i..].iter().take(4 * 16);

		let last = (i + 64) > total_length;
		// technically could clone original slice and add this on, but instead
		let iter = if let Some(slice) = input.get(i..) {
			if slice.len() < 64 {
				// assert!(slice.len() < 64);
				Iterator::chain(slice.iter().take(64), &[0b1000_0000])
			} else {
				Iterator::chain(slice.iter().take(64), &[])
			}
		} else {
			// dbg!("here");
			// A rare case
			// assert!(last);
			// assert!(total_length % 64 > 56, "i={i} il={il} tl={total_length}", il=input.len());
			static EMPTY: &[u8] = &[];
			Iterator::chain(EMPTY.iter().take(64), &[])
		};

		for (j, byte) in iter.enumerate() {
			let offset = (3 - j % 4) * 8;
			let u32_byte = *byte as u32;
			chunk[j / 4] |= u32_byte << offset;
		}

		if last {
			let ml: u64 = input.len() as u64 * 8;
			let [a, b, c, d, e, f, g, h] = ml.to_be_bytes();
			chunk[14] = u32::from_be_bytes([a, b, c, d]);
			chunk[15] = u32::from_be_bytes([e, f, g, h]);
		}

		// Message schedule: extend the sixteen 32-bit words into eighty 32-bit words:
		for j in 16..80 {
			let temp = chunk[j - 3] ^ chunk[j - 8] ^ chunk[j - 14] ^ chunk[j - 16];
			// Note 3: SHA-0 differs by not having this leftrotate.
			chunk[j] = u32::rotate_left(temp, 1);
		}

		let mut a: u32 = h0;
		let mut b: u32 = h1;
		let mut c: u32 = h2;
		let mut d: u32 = h3;
		let mut e: u32 = h4;

		for j in 0..80 {
			let (f, k) = match j {
				0..=19 => ((b & c) | (!b & d), 0x5A827999),
				20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
				40..=59 => (b & c | b & d | c & d, 0x8F1BBCDC),
				_ => (b ^ c ^ d, 0xCA62C1D6),
			};

			let temp = a.rotate_left(5);
			let temp = temp.wrapping_add(f);
			let temp = temp.wrapping_add(e);
			let temp = temp.wrapping_add(k);
			let temp = temp.wrapping_add(chunk[j]);

			e = d;
			d = c;
			c = b.rotate_left(30);
			b = a;
			a = temp;
		}

		h0 = h0.wrapping_add(a);
		h1 = h1.wrapping_add(b);
		h2 = h2.wrapping_add(c);
		h3 = h3.wrapping_add(d);
		h4 = h4.wrapping_add(e);

		i += 64;
	}

	let source: [u32; 5] = [h0, h1, h2, h3, h4];
	let mut out = [0u8; 20];

	// Split the result array into mutable 4-byte chunks and zip with the u32s
	for (chunk, hi) in out.chunks_exact_mut(4).zip(source.iter()) {
		chunk.copy_from_slice(&hi.to_be_bytes());
	}

	out
}
