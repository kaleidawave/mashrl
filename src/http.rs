use std::borrow::Cow;

/// Expecting [Response]. Lifetime for using current program data in generation (not result).
pub struct Request<'a, B> {
	pub method: Method<'a>,
	pub path: Cow<'a, str>,
	pub headers: Headers<'a>,
	pub body: B,
}

/// The out of a [Request]. Lifetime for using current program data in generation (not result).
pub struct Response<'a, B> {
	pub code: ResponseCode,
	pub headers: Headers<'a>,
	pub body: B,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Method<'a>(pub Cow<'a, str>);

impl Method<'static> {
	pub const GET: Self = Method(Cow::Borrowed("GET"));
	pub const HEAD: Self = Method(Cow::Borrowed("HEAD"));
	pub const POST: Self = Method(Cow::Borrowed("POST"));
	pub const PUT: Self = Method(Cow::Borrowed("PUT"));
	pub const PATCH: Self = Method(Cow::Borrowed("PATCH"));
	pub const CONNECT: Self = Method(Cow::Borrowed("CONNECT"));
	pub const TRACE: Self = Method(Cow::Borrowed("TRACE"));
}

pub struct Body<B>(pub(crate) B);
// pub struct Body<B>(pub(crate) B);

impl<'a> Request<'a, std::io::Empty> {
	#[must_use]
	pub fn new_get(path: &'a str, headers: Headers<'a>) -> Self {
		Self { method: Method::GET, headers, path: Cow::Borrowed(path), body: std::io::empty() }
	}
}

// TODO add
// impl<'a> Request<'a, std::io::Cursor<>> {
// 	pub fn new_post(path: &'a str, headers: Headers<'a>) -> Self {
// 		Self {
// 			method: Method::GET,
// 			headers: headers,
// 			path: Cow::Borrowed(path),
// 			body: std::io::empty()
// 		}
// 	}
// }

// impl<B: std::io::Read + 'static> Body<B> {
//     pub fn into_dyn(self) -> Body<Box<dyn std::io::Read>> {
//         Body(Box::new(self.0))
//     }
// }

impl<B: std::io::Read> Body<B> {
	pub fn new(reader: B) -> Self {
		Self(reader)
	}

	pub fn raw(self) -> B {
		self.0
	}

	pub fn get_reader(self, headers: &Headers) -> ResponseBody<B> {
		let mut reader = ResponseBody::Base(self.0);

		if let Some(transfer_encoding) = headers.get("Transfer-Encoding") {
			for part in transfer_encoding.split(',').map(str::trim) {
				match part {
					"chunked" => {
						let buf_reader = std::io::BufReader::new(Box::new(reader));
						let chunked_reader = chunked::ChunkedReader::new(buf_reader);
						reader = ResponseBody::Chunked(chunked_reader);
					}
					#[cfg(feature = "decompress")]
					"gzip" => {
						let gzip_reader = flate2::read::GzDecoder::new(Box::new(reader));
						reader = ResponseBody::Gzipped(gzip_reader);
					}
					part => {
						eprintln!("Unhandled encoding {part:?}");
					}
				}
			}
		}

		if let Some(content_encoding) = headers.get("Content-Encoding") {
			for part in content_encoding.split(',').map(str::trim) {
				match part {
					#[cfg(feature = "decompress")]
					"gzip" => {
						let gzip_reader = flate2::read::GzDecoder::new(Box::new(reader));
						reader = ResponseBody::Gzipped(gzip_reader);
					}
					part => {
						eprintln!("Unhandled encoding {part:?}");
					}
				}
			}
		}

		reader
	}
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ResponseCode(pub u16);

impl ResponseCode {
	pub const CONTINUE: ResponseCode = ResponseCode(100);
	pub const SWITCHING_PROTOCOLS: ResponseCode = ResponseCode(101);
	pub const PROCESSING: ResponseCode = ResponseCode(102);
	pub const EARLY_HINTS: ResponseCode = ResponseCode(103);

	pub const OK: ResponseCode = ResponseCode(200);
	pub const CREATED: ResponseCode = ResponseCode(201);
	pub const ACCEPTED: ResponseCode = ResponseCode(202);
	pub const NON_AUTHORITATIVE_INFORMATION: ResponseCode = ResponseCode(203);
	pub const NO_CONTENT: ResponseCode = ResponseCode(204);
	pub const RESET_CONTENT: ResponseCode = ResponseCode(205);
	pub const PARTIAL_CONTENT: ResponseCode = ResponseCode(206);
	pub const MULTI_STATUS: ResponseCode = ResponseCode(207);
	pub const ALREADY_REPORTED: ResponseCode = ResponseCode(208);
	pub const IM_USED: ResponseCode = ResponseCode(226);

	pub const MULTIPLE_CHOICES: ResponseCode = ResponseCode(300);
	pub const MOVED_PERMANENTLY: ResponseCode = ResponseCode(301);
	pub const FOUND: ResponseCode = ResponseCode(302);
	pub const SEE_OTHER: ResponseCode = ResponseCode(303);
	pub const NOT_MODIFIED: ResponseCode = ResponseCode(304);
	pub const TEMPORARY_REDIRECT: ResponseCode = ResponseCode(307);
	pub const PERMANENT_REDIRECT: ResponseCode = ResponseCode(308);

	pub const BAD_REQUEST: ResponseCode = ResponseCode(400);
	pub const UNAUTHORIZED: ResponseCode = ResponseCode(401);
	pub const PAYMENT_REQUIRED: ResponseCode = ResponseCode(402);
	pub const FORBIDDEN: ResponseCode = ResponseCode(403);
	pub const NOT_FOUND: ResponseCode = ResponseCode(404);
	pub const METHOD_NOT_ALLOWED: ResponseCode = ResponseCode(405);
	pub const NOT_ACCEPTABLE: ResponseCode = ResponseCode(406);
	pub const PROXY_AUTHENTICATION_REQUIRED: ResponseCode = ResponseCode(407);
	pub const REQUEST_TIMEOUT: ResponseCode = ResponseCode(408);
	pub const CONFLICT: ResponseCode = ResponseCode(409);
	pub const GONE: ResponseCode = ResponseCode(410);
	pub const LENGTH_REQUIRED: ResponseCode = ResponseCode(411);
	pub const PRECONDITION_FAILED: ResponseCode = ResponseCode(412);
	pub const CONTENT_TOO_LARGE: ResponseCode = ResponseCode(413);
	pub const URI_TOO_LONG: ResponseCode = ResponseCode(414);
	pub const UNSUPPORTED_MEDIA_TYPE: ResponseCode = ResponseCode(415);
	pub const RANGE_NOT_SATISFIABLE: ResponseCode = ResponseCode(416);
	pub const EXPECTATION_FAILED: ResponseCode = ResponseCode(417);
	pub const IM_A_TEAPOT: ResponseCode = ResponseCode(418);
	pub const MISDIRECTED_REQUEST: ResponseCode = ResponseCode(421);
	pub const UNPROCESSABLE_CONTENT: ResponseCode = ResponseCode(422);
	pub const LOCKED: ResponseCode = ResponseCode(423);
	pub const FAILED_DEPENDENCY: ResponseCode = ResponseCode(424);
	pub const TOO_EARLY: ResponseCode = ResponseCode(425);
	pub const UPGRADE_REQUIRED: ResponseCode = ResponseCode(426);
	pub const PRECONDITION_REQUIRED: ResponseCode = ResponseCode(428);
	pub const TOO_MANY_REQUESTS: ResponseCode = ResponseCode(429);
	pub const REQUEST_HEADER_FIELDS_TOO_LARGE: ResponseCode = ResponseCode(431);
	pub const UNAVAILABLE_FOR_LEGAL_REASONS: ResponseCode = ResponseCode(451);

	pub const INTERNAL_SERVER_ERROR: ResponseCode = ResponseCode(500);
	pub const NOT_IMPLEMENTED: ResponseCode = ResponseCode(501);
	pub const BAD_GATEWAY: ResponseCode = ResponseCode(502);
	pub const SERVICE_UNAVAILABLE: ResponseCode = ResponseCode(503);
	pub const GATEWAY_TIMEOUT: ResponseCode = ResponseCode(504);
	pub const HTTP_VERSION_NOT_SUPPORTED: ResponseCode = ResponseCode(505);
	pub const VARIANT_ALSO_NEGOTIATES: ResponseCode = ResponseCode(506);
	pub const INSUFFICIENT_STORAGE: ResponseCode = ResponseCode(507);
	pub const LOOP_DETECTED: ResponseCode = ResponseCode(508);
	pub const NOT_EXTENDED: ResponseCode = ResponseCode(510);
	pub const NETWORK_AUTHENTICATION_REQUIRED: ResponseCode = ResponseCode(511);

	/// # Errors
	///
	/// returns an `Err` if the input is not a known HTTP response code
	pub fn from_line(item: &str) -> Result<Self, &str> {
		Ok(match item {
			"100 Continue" => Self::CONTINUE,
			"101 Switching Protocols" => Self::SWITCHING_PROTOCOLS,
			"102 Processing" => Self::PROCESSING,
			"103 Early Hints" => Self::EARLY_HINTS,
			"200 OK" => Self::OK,
			"201 Created" => Self::CREATED,
			"202 Accepted" => Self::ACCEPTED,
			"203 Non-Authoritative Information" => Self::NON_AUTHORITATIVE_INFORMATION,
			"204 No Content" => Self::NO_CONTENT,
			"205 Reset Content" => Self::RESET_CONTENT,
			"206 Partial Content" => Self::PARTIAL_CONTENT,
			"207 Multi-Status" => Self::MULTI_STATUS,
			"208 Already Reported" => Self::ALREADY_REPORTED,
			"226 IM Used" => Self::IM_USED,
			"300 Multiple Choices" => Self::MULTIPLE_CHOICES,
			"301 Moved Permanently" => Self::MOVED_PERMANENTLY,
			"302 Found" => Self::FOUND,
			"303 See Other" => Self::SEE_OTHER,
			"304 Not Modified" => Self::NOT_MODIFIED,
			"307 Temporary Redirect" => Self::TEMPORARY_REDIRECT,
			"308 Permanent Redirect" => Self::PERMANENT_REDIRECT,
			"400 Bad Request" => Self::BAD_REQUEST,
			"401 Unauthorized" => Self::UNAUTHORIZED,
			"402 Payment Required" => Self::PAYMENT_REQUIRED,
			"403 Forbidden" => Self::FORBIDDEN,
			"404 Not Found" => Self::NOT_FOUND,
			"405 Method Not Allowed" => Self::METHOD_NOT_ALLOWED,
			"406 Not Acceptable" => Self::NOT_ACCEPTABLE,
			"407 Proxy Authentication Required" => Self::PROXY_AUTHENTICATION_REQUIRED,
			"408 Request Timeout" => Self::REQUEST_TIMEOUT,
			"409 Conflict" => Self::CONFLICT,
			"410 Gone" => Self::GONE,
			"411 Length Required" => Self::LENGTH_REQUIRED,
			"412 Precondition Failed" => Self::PRECONDITION_FAILED,
			"413 Content Too Large" => Self::CONTENT_TOO_LARGE,
			"414 URI Too Long" => Self::URI_TOO_LONG,
			"415 Unsupported Media Type" => Self::UNSUPPORTED_MEDIA_TYPE,
			"416 Range Not Satisfiable" => Self::RANGE_NOT_SATISFIABLE,
			"417 Expectation Failed" => Self::EXPECTATION_FAILED,
			"418 I'm a teapot" => Self::IM_A_TEAPOT,
			"421 Misdirected Request" => Self::MISDIRECTED_REQUEST,
			"422 Unprocessable Content" => Self::UNPROCESSABLE_CONTENT,
			"423 Locked" => Self::LOCKED,
			"424 Failed Dependency" => Self::FAILED_DEPENDENCY,
			"425 Too Early" => Self::TOO_EARLY,
			"426 Upgrade Required" => Self::UPGRADE_REQUIRED,
			"428 Precondition Required" => Self::PRECONDITION_REQUIRED,
			"429 Too Many Requests" => Self::TOO_MANY_REQUESTS,
			"431 Request Header Fields Too Large" => Self::REQUEST_HEADER_FIELDS_TOO_LARGE,
			"451 Unavailable For Legal Reasons" => Self::UNAVAILABLE_FOR_LEGAL_REASONS,
			"500 Internal Server Error" => Self::INTERNAL_SERVER_ERROR,
			"501 Not Implemented" => Self::NOT_IMPLEMENTED,
			"502 Bad Gateway" => Self::BAD_GATEWAY,
			"503 Service Unavailable" => Self::SERVICE_UNAVAILABLE,
			"504 Gateway Timeout" => Self::GATEWAY_TIMEOUT,
			"505 HTTP Version Not Supported" => Self::HTTP_VERSION_NOT_SUPPORTED,
			"506 Variant Also Negotiates" => Self::VARIANT_ALSO_NEGOTIATES,
			"507 Insufficient Storage" => Self::INSUFFICIENT_STORAGE,
			"508 Loop Detected" => Self::LOOP_DETECTED,
			"510 Not Extended" => Self::NOT_EXTENDED,
			"511 Network Authentication Required" => Self::NETWORK_AUTHENTICATION_REQUIRED,
			item => {
				// Custom status code
				if let Some(value) =
					item.split_once(' ').and_then(|(lhs, _)| lhs.parse::<u16>().ok())
				{
					Self(value)
				} else {
					return Err(item);
				}
			}
		})
	}

	/// # Panics
	///
	/// TODO Panics on custom HTTP codes?
	#[must_use]
	pub fn to_str(&self) -> &str {
		match *self {
			Self::CONTINUE => "100 Continue",
			Self::SWITCHING_PROTOCOLS => "101 Switching Protocols",
			Self::PROCESSING => "102 Processing",
			Self::EARLY_HINTS => "103 Early Hints",
			Self::OK => "200 OK",
			Self::CREATED => "201 Created",
			Self::ACCEPTED => "202 Accepted",
			Self::NON_AUTHORITATIVE_INFORMATION => "203 Non-Authoritative Information",
			Self::NO_CONTENT => "204 No Content",
			Self::RESET_CONTENT => "205 Reset Content",
			Self::PARTIAL_CONTENT => "206 Partial Content",
			Self::MULTI_STATUS => "207 Multi-Status",
			Self::ALREADY_REPORTED => "208 Already Reported",
			Self::IM_USED => "226 IM Used",
			Self::MULTIPLE_CHOICES => "300 Multiple Choices",
			Self::MOVED_PERMANENTLY => "301 Moved Permanently",
			Self::FOUND => "302 Found",
			Self::SEE_OTHER => "303 See Other",
			Self::NOT_MODIFIED => "304 Not Modified",
			Self::TEMPORARY_REDIRECT => "307 Temporary Redirect",
			Self::PERMANENT_REDIRECT => "308 Permanent Redirect",
			Self::BAD_REQUEST => "400 Bad Request",
			Self::UNAUTHORIZED => "401 Unauthorized",
			Self::PAYMENT_REQUIRED => "402 Payment Required",
			Self::FORBIDDEN => "403 Forbidden",
			Self::NOT_FOUND => "404 Not Found",
			Self::METHOD_NOT_ALLOWED => "405 Method Not Allowed",
			Self::NOT_ACCEPTABLE => "406 Not Acceptable",
			Self::PROXY_AUTHENTICATION_REQUIRED => "407 Proxy Authentication Required",
			Self::REQUEST_TIMEOUT => "408 Request Timeout",
			Self::CONFLICT => "409 Conflict",
			Self::GONE => "410 Gone",
			Self::LENGTH_REQUIRED => "411 Length Required",
			Self::PRECONDITION_FAILED => "412 Precondition Failed",
			Self::CONTENT_TOO_LARGE => "413 Content Too Large",
			Self::URI_TOO_LONG => "414 URI Too Long",
			Self::UNSUPPORTED_MEDIA_TYPE => "415 Unsupported Media Type",
			Self::RANGE_NOT_SATISFIABLE => "416 Range Not Satisfiable",
			Self::EXPECTATION_FAILED => "417 Expectation Failed",
			Self::IM_A_TEAPOT => "418 I'm a teapot",
			Self::MISDIRECTED_REQUEST => "421 Misdirected Request",
			Self::UNPROCESSABLE_CONTENT => "422 Unprocessable Content",
			Self::LOCKED => "423 Locked",
			Self::FAILED_DEPENDENCY => "424 Failed Dependency",
			Self::TOO_EARLY => "425 Too Early",
			Self::UPGRADE_REQUIRED => "426 Upgrade Required",
			Self::PRECONDITION_REQUIRED => "428 Precondition Required",
			Self::TOO_MANY_REQUESTS => "429 Too Many Requests",
			Self::REQUEST_HEADER_FIELDS_TOO_LARGE => "431 Request Header Fields Too Large",
			Self::UNAVAILABLE_FOR_LEGAL_REASONS => "451 Unavailable For Legal Reasons",
			Self::INTERNAL_SERVER_ERROR => "500 Internal Server Error",
			Self::NOT_IMPLEMENTED => "501 Not Implemented",
			Self::BAD_GATEWAY => "502 Bad Gateway",
			Self::SERVICE_UNAVAILABLE => "503 Service Unavailable",
			Self::GATEWAY_TIMEOUT => "504 Gateway Timeout",
			Self::HTTP_VERSION_NOT_SUPPORTED => "505 HTTP Version Not Supported",
			Self::VARIANT_ALSO_NEGOTIATES => "506 Variant Also Negotiates",
			Self::INSUFFICIENT_STORAGE => "507 Insufficient Storage",
			Self::LOOP_DETECTED => "508 Loop Detected",
			Self::NOT_EXTENDED => "510 Not Extended",
			Self::NETWORK_AUTHENTICATION_REQUIRED => "511 Network Authentication Required",
			item => {
				todo!("custom code for {item:?}")
			}
		}
	}
}

impl<B: std::io::Read> Response<'_, Body<B>> {
	/// seperate method because it takes ownership of reading
	pub fn debug(self) -> impl std::fmt::Debug {
		let body = {
			use std::io::Read;

			// TODO more here
			let mut body = String::new();
			let mut reader = self.body.get_reader(&self.headers);
			reader.read_to_string(&mut body).unwrap();
			body
		};

		std::fmt::from_fn(move |fmt| {
			let mut fmt = fmt.debug_struct("Response");
			fmt.field("code", &self.code);
			fmt.field("headers", &self.headers);
			fmt.field("body", &body);
			fmt.finish()
		})
	}
}

#[derive(Clone)]
pub struct Headers<'a>(pub Cow<'a, str>);

impl Headers<'static> {
	#[must_use]
	pub fn empty() -> Headers<'static> {
		Headers(Cow::Borrowed(""))
	}

	/// # Errors
	///
	/// returns an `Err` if the input is not `utf8`
	pub fn from_raw(on: Vec<u8>) -> Result<Headers<'static>, std::string::FromUtf8Error> {
		String::from_utf8(on).map(Headers::from_string)
	}

	#[must_use]
	pub fn from_string(on: String) -> Headers<'static> {
		Headers(Cow::Owned(on))
	}
}

impl Headers<'_> {
	#[must_use]
	pub fn iter(&self) -> HeaderIter<'_> {
		HeaderIter(self.0.lines())
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	#[must_use]
	pub fn is_valid(&self) -> bool {
		self.is_empty() || self.0.ends_with("\r\n")
	}

	#[must_use]
	pub fn get(&self, key: &str) -> Option<&str> {
		self.iter().find_map(|(key2, value)| (key.eq_ignore_ascii_case(key2)).then_some(value))
	}

	pub fn set(&mut self, key: &str, value: &str) {
		let mut position = None;
		let this: &str = &self.0;
		let offset = this.as_ptr() as usize;
		for line in this.lines() {
			if let Some((key2, _)) = line.split_once(':')
				&& key.eq_ignore_ascii_case(key2)
			{
				let start = line.as_ptr() as usize - offset;
				position = Some((start, start + line.len() + 2));
				break;
			}
		}
		if let Some((start, end)) = position {
			let buf = self.0.to_mut();
			buf.drain(start..end);
		}
		self.append(key, value);
	}

	pub fn append(&mut self, key: &str, value: &str) {
		let buf = self.0.to_mut();
		buf.push_str(key);
		buf.push_str(": ");
		buf.push_str(value);
		buf.push_str("\r\n");
	}

	pub fn delete(&mut self, key: &str) {
		let mut position = None;
		let this: &str = &self.0;
		let offset = this.as_ptr() as usize;
		for line in this.lines() {
			if let Some((key2, _)) = line.split_once(':')
				&& key.eq_ignore_ascii_case(key2)
			{
				let start = line.as_ptr() as usize - offset;
				position = Some((start, start + line.len() + 2));
				break;
			}
		}
		if let Some((start, end)) = position {
			let buf = self.0.to_mut();
			buf.drain(start..end);
		}
	}
}

impl std::fmt::Debug for Headers<'_> {
	fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut fmt = fmt.debug_struct("Headers");
		for (key, value) in self {
			fmt.field(key, &value);
		}
		fmt.finish()
	}
}

impl<T: AsRef<str>> FromIterator<(T, T)> for Headers<'static> {
	fn from_iter<I: IntoIterator<Item = (T, T)>>(iter: I) -> Self {
		let mut buf = String::new();
		for (key, value) in iter {
			buf.push_str(key.as_ref());
			buf.push_str(": ");
			buf.push_str(value.as_ref());
			buf.push_str("\r\n");
		}
		Self::from_string(buf)
	}
}

impl<'a> IntoIterator for &'a Headers<'_> {
	type Item = (&'a str, &'a str);
	type IntoIter = HeaderIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		HeaderIter(self.0.lines())
	}
}

pub struct HeaderIter<'a>(pub(super) std::str::Lines<'a>);

impl<'a> Iterator for HeaderIter<'a> {
	type Item = (&'a str, &'a str);

	fn next(&mut self) -> Option<Self::Item> {
		let row = self.0.next()?;
		// TODO what if `None` here?
		let (key, value) = row.split_once(':')?;
		Some((key, value.trim()))
	}
}

pub mod chunked {
	pub struct ChunkedReader<T> {
		reader: T,
		to_read: usize,
	}

	impl<T: std::io::Read> ChunkedReader<T> {
		pub fn new(reader: T) -> Self {
			Self { reader, to_read: 0 }
		}

		pub fn into_inner(self) -> T {
			self.reader
		}
	}

	impl<T: std::io::BufRead> std::io::Read for ChunkedReader<T> {
		fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
			if self.to_read == 0 {
				let mut chunk_size_buf = String::new();
				self.reader.read_line(&mut chunk_size_buf)?;

				debug_assert!(chunk_size_buf.ends_with("\r\n"));
				let chunk_size_str = chunk_size_buf.trim_end();
				let hex = u64::from_str_radix(chunk_size_str, 16);
				let Ok(chunk_size) = hex else {
					let message = format!("invalid chunk length {chunk_size_str:?}");
					let error = std::io::Error::new(std::io::ErrorKind::InvalidData, message);
					return Err(error);
				};

				// The terminating chunk is a zero-length chunk
				if chunk_size == 0 {
					return Ok(0);
				}

				// Convert u64 -> usize ?
				let chunk_size = usize::try_from(chunk_size).unwrap_or(usize::MAX);

				self.to_read = chunk_size;
			}

			let mut reader_over_chunk = self.reader.by_ref().take(self.to_read as u64);
			let bytes_tranferred = reader_over_chunk.read(buf)?;
			self.to_read -= bytes_tranferred;

			if self.to_read == 0 {
				let mut end: [u8; 2] = [0, 0];
				self.reader.read_exact(&mut end)?;

				// #[cfg(debug_assertions)]
				if &end != b"\r\n" {
					let message = "expected '\r\n' at end of chunked frame";
					let error = std::io::Error::new(std::io::ErrorKind::InvalidData, message);
					return Err(error);
				}
			}

			Ok(bytes_tranferred)
		}
	}

	// pub struct ChunkedWriter<T> {
	//     writer: T
	// }

	// impl<T: std::io::Write> ChunkedWriter<T> {
	// 	pub fn new(writer: T) -> Self {
	// 		Self { writer }
	// 	}
	// }

	// impl<T: std::io::Write> std::io::Write for ChunkedWriter<T> {
	// 	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
	// 		write!(&mut self.writer, "{len:x}\r\n", len=buf.len())?;
	// 		write!(&mut self.writer, "{buf}")
	// 	}

	//     fn flush(&mut self) -> std::io::Result<()> {
	// 		self.writer.flush()
	// 	}
	// }
	//

	pub struct ChunkedIterator<I> {
		iter: I,
		buf: Vec<u8>,
		finished: bool,
	}

	impl<I> ChunkedIterator<I> {
		pub fn new(iter: I) -> Self {
			Self { iter, buf: Vec::new(), finished: false }
		}
	}

	// TODO cow
	// TODO part sizes
	impl<I: Iterator<Item = Vec<u8>>> std::io::Read for ChunkedIterator<I> {
		fn read(&mut self, write_to: &mut [u8]) -> std::io::Result<usize> {
			use std::io::Write;

			let mut written = 0;
			if !self.buf.is_empty() {
				let to_remove = std::cmp::min(self.buf.len(), write_to.len());
				for value in self.buf.drain(..to_remove) {
					write_to[written] = value;
					written += 1;
				}
			}

			// TODO 2
			if let Some(remaining) = write_to.len().checked_sub(written)
				&& remaining > 2
				&& !self.finished
			{
				// Last is zero
				let next = self.iter.next();
				self.finished = next.is_none();
				let mut next = next.unwrap_or_default();
				// TODO faster
				let data_len = next.len();
				let mut chunk = Vec::new();
				write!(&mut chunk, "{data_len:x}\r\n").unwrap();
				chunk.append(&mut next);
				write!(&mut chunk, "\r\n").unwrap();
				// TODO
				let to_add = chunk.drain(..std::cmp::min(chunk.len(), remaining));
				for value in to_add {
					write_to[written] = value;
					written += 1;
				}
				self.buf = chunk;
			}

			Ok(written)
		}
	}
}

// TODO brotli?
/// Processed body
pub enum ResponseBody<T> {
	Base(T),
	// Limited(std::io::Take<InitialBody>),
	/// TODO just `B` rather `ResponseBody`
	Chunked(chunked::ChunkedReader<std::io::BufReader<Box<ResponseBody<T>>>>),
	#[cfg(feature = "decompress")]
	Gzipped(flate2::read::GzDecoder<Box<ResponseBody<T>>>),
	#[cfg(feature = "decompress")]
	Deflate(flate2::read::DeflateDecoder<Box<ResponseBody<T>>>),
}

impl<B: std::io::Read> ResponseBody<B> {
	#[must_use]
	pub fn into_inner(self) -> Self {
		match self {
			// ResponseBody::Empty => self,
			ResponseBody::Base(ref _inner) => self,
			// ResponseBody::Limited(ref _inner) => self,
			// ResponseBody::Custom(ref _inner) => self,
			ResponseBody::Chunked(inner) => *inner.into_inner().into_inner(),
			#[cfg(feature = "decompress")]
			ResponseBody::Gzipped(inner) => inner.into_inner().into_inner(),
			#[cfg(feature = "decompress")]
			ResponseBody::Deflate(inner) => inner.into_inner().into_inner(),
		}
	}
}

impl<B: std::io::Read> std::io::Read for ResponseBody<B> {
	// impl std::io::Read for ResponseBody {
	fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
		match self {
			// Self::Empty => Ok(0),
			Self::Base(inner) => std::io::Read::read(inner, into),
			Self::Chunked(inner) => std::io::Read::read(inner, into),
			// Self::Limited(inner) => std::io::Read::read(inner, into),
			// Self::Custom(inner) => std::io::Read::read(inner, into),
			#[cfg(feature = "decompress")]
			Self::Gzipped(inner) => std::io::Read::read(inner, into),
			#[cfg(feature = "decompress")]
			Self::Deflate(inner) => std::io::Read::read(inner, into),
		}
	}
}
