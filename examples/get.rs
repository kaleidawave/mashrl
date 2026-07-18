use mashrl::{self, http};

fn main() {
	// let response = make_get_request("httpbin.org", "get", http::Headers::empty()).unwrap();

	{
		let headers = http::Headers::from_iter([
			("Host", "github.com"),
			// ("User-Agent", "git/2.52.0"),
			// ("Accept", "*/*"),
			// ("Accept-Encoding", "deflate, gzip"),
			// ("Accept-Language", "C, *;q=0.9"),
			// ("Pragma", "no-cache"),
			// ("Git-Protocol", "version=2"),
		]);

		let request = http::Request {
			method: http::Method::GET,
			path: "/kaleidawave/depict.git/info/refs?service=git-upload-pack".into(),
			headers,
			body: std::io::empty(),
		};

		let response = mashrl::make_request(request).unwrap();

		dbg!(response.code);
		dbg!(&response.headers);

		{
			use std::io::Read;

			let mut buf = String::new();
			let mut body = response.body.get_reader(&response.headers); // .raw(); // .take(4);
			let n = body.read_to_string(&mut buf).unwrap();

			dbg!(n);
			println!("{buf}");
		}

		// dbg!(response.body);

		// dbg!(response.debug());
	}
	// dbg!(&headers.0);
	// let headers = http::Headers::empty();
	// let response = make_get_request(
	//     "github.com",
	//     "kaleidawave/depict.git/info/refs?service=git-upload-pack",
	//     headers,
	// )
	// .unwrap();

	// eprintln!("Code {code}", code = response.code.0);

	// for (key, value) in &response.headers {
	//     eprintln!("{key}: {value}");
	// }

	// let mut content = String::new();
	// response.body.read_to_string(&mut content).unwrap();
	// eprintln!("{content}");
}
