use crate::http;

pub type DynRead = Box<dyn std::io::Read + Send>;

pub fn get(url: &str) -> std::io::Result<String> {
	let url = if let Some(url) = url.strip_prefix("https://") { url } else { url };
	let first = url.find('/').unwrap();
	let (host, path) = url.split_at(first);
	let headers = http::Headers::from_iter([
		("Host", host),
		("User-Agent", "mashrl"),
		// ("Accept", "*/*"),
		// ("Accept-Encoding", "deflate, gzip"),
		// ("Accept-Language", "C, *;q=0.9"),
		// ("Pragma", "no-cache"),
		// ("Git-Protocol", "version=2"),
	]);

	let request = http::Request {
		method: http::Method::GET,
		path: path.into(),
		headers,
		body: std::io::empty(),
	};

	let response = super::make_request(request).unwrap();

	std::io::read_to_string(response.body.get_reader(&response.headers))
}

#[cfg(all(feature = "extras", feature = "server"))]
pub fn serve_file(path: &std::path::Path) -> http::Response<'static, DynRead> {
	let mut headers = http::Headers::empty();

	if let Some(extension) = path.extension().and_then(std::ffi::OsStr::to_str) {
		headers.append(
			"Content-Type",
			crate::server::mimetypes::get_mimetype_from_extension(extension),
		);
	}

	headers.append("Connection", "close");
	if let Ok(file) = std::fs::File::open(path) {
		let size = file.metadata().unwrap().len();
		headers.append("Content-Length", &size.to_string());
		http::Response { code: http::ResponseCode::OK, headers, body: Box::new(file) }
	} else {
		http::Response {
			code: http::ResponseCode::NOT_FOUND,
			headers,
			body: Box::new(std::io::empty()),
		}
	}
}

#[cfg(all(feature = "extras", feature = "server"))]
pub fn serve_file_dangerous_across_origin(
	path: &std::path::Path,
) -> http::Response<'static, DynRead> {
	let mut headers = http::Headers::empty();

	if let Some(extension) = path.extension().and_then(std::ffi::OsStr::to_str) {
		headers.append(
			"Content-Type",
			crate::server::mimetypes::get_mimetype_from_extension(extension),
		);
	}

	headers.append("Connection", "close");
	headers.append("Access-Control-Allow-Origin", "*");

	if let Ok(file) = std::fs::File::open(path) {
		let size = file.metadata().unwrap().len();
		headers.append("Content-Length", &size.to_string());
		http::Response { code: http::ResponseCode::OK, headers, body: Box::new(file) }
	} else {
		http::Response {
			code: http::ResponseCode::NOT_FOUND,
			headers,
			body: Box::new(std::io::empty()),
		}
	}
}
