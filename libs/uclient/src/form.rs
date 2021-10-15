use concat_reader::ConcatRead;
use http::header::HeaderMap;
use mime_multipart::{generate_boundary, get_multipart_boundary, Node, Part};
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};

fn prepare_u8s(
    content: Vec<u8>,
    count: &mut usize,
    readers: &mut Vec<Box<dyn Read + Send + Sync>>,
) {
    *count += content.len();
    readers.push(Box::new(Cursor::new(content)));
}

/// convert a multipart node to an `std::io::Read` and the bytes count
pub fn multipart_to_read(
    boundary: Vec<u8>,
    nodes: Vec<Node>,
) -> Result<(usize, impl ConcatRead + Send + Sync), crate::Error> {
    let mut count: usize = 0;
    let mut readers: Vec<Box<dyn Read + Send + Sync>> = vec![];

    for node in nodes {
        // write a boundary
        prepare_u8s(b"--".to_vec(), &mut count, &mut readers);
        prepare_u8s(boundary.clone(), &mut count, &mut readers);
        prepare_u8s(b"\r\n".to_vec(), &mut count, &mut readers);

        match node {
            Node::Part(part) => {
                // write the part's headers
                for header in part.headers.iter() {
                    prepare_u8s(header.name().as_bytes().to_vec(), &mut count, &mut readers);
                    prepare_u8s(b": ".to_vec(), &mut count, &mut readers);
                    prepare_u8s(
                        header.value_string().as_bytes().to_vec(),
                        &mut count,
                        &mut readers,
                    );
                    prepare_u8s(b"\r\n".to_vec(), &mut count, &mut readers);
                }

                // write the blank line
                prepare_u8s(b"\r\n".to_vec(), &mut count, &mut readers);

                // write the part's content
                prepare_u8s(part.body, &mut count, &mut readers);
            }
            Node::File(filepart) => {
                // write the part's headers
                for header in filepart.headers.iter() {
                    prepare_u8s(header.name().as_bytes().to_vec(), &mut count, &mut readers);
                    prepare_u8s(b": ".to_vec(), &mut count, &mut readers);
                    prepare_u8s(
                        header.value_string().as_bytes().to_vec(),
                        &mut count,
                        &mut readers,
                    );
                    prepare_u8s(b"\r\n".to_vec(), &mut count, &mut readers);
                }

                // write the blank line
                prepare_u8s(b"\r\n".to_vec(), &mut count, &mut readers);

                // write out the files's content
                let f = File::open(&filepart.path).map_err(|_| crate::Error::InvalidFile)?;

                let meta = f.metadata().map_err(|_| crate::Error::InvalidFile)?;
                count += meta.len() as usize;

                let file = BufReader::new(f);
                readers.push(Box::new(file));
            }
            Node::Multipart((headers, subnodes)) => {
                // get boundary
                let boundary = get_multipart_boundary(&headers)?;

                // write the multipart headers
                for header in headers.iter() {
                    prepare_u8s(header.name().as_bytes().to_vec(), &mut count, &mut readers);
                    prepare_u8s(b": ".to_vec(), &mut count, &mut readers);
                    prepare_u8s(
                        header.value_string().as_bytes().to_vec(),
                        &mut count,
                        &mut readers,
                    );
                    prepare_u8s(b"\r\n".to_vec(), &mut count, &mut readers);
                }

                // write the blank line
                prepare_u8s(b"\r\n".to_vec(), &mut count, &mut readers);

                // recurse
                multipart_to_read(boundary.clone(), subnodes)?;
            }
        }

        // write a line terminator
        prepare_u8s(b"\r\n".to_vec(), &mut count, &mut readers);
    }

    // write a final boundary
    prepare_u8s(b"--".to_vec(), &mut count, &mut readers);
    prepare_u8s(boundary.clone(), &mut count, &mut readers);
    prepare_u8s(b"--".to_vec(), &mut count, &mut readers);

    let reader = concat_reader::concat(readers);
    Ok((count, reader))
}

/// A file in multipart
///
/// A file that is to be inserted into a `multipart/*` or alternatively an uploaded file that
/// was received as part of `multipart/*` parsing.
#[derive(Clone, Debug, PartialEq)]
pub struct FilePart {
    /// The headers of the part
    pub headers: HeaderMap,
    /// A temporary file containing the file content
    pub path: PathBuf,
    /// Optionally, the size of the file.  This is filled when multiparts are parsed, but is
    /// not necessary when they are generated.
    pub size: Option<usize>,
}
impl FilePart {
    pub fn new(headers: HeaderMap, path: &Path) -> FilePart {
        FilePart {
            headers: headers,
            path: path.to_owned(),
            size: None,
        }
    }
}
impl Into<mime_multipart::FilePart> for FilePart {
    fn into(self) -> mime_multipart::FilePart {
        use hyper::header::Headers;
        let mut h = Headers::new();
        for (k, v) in self.headers.into_iter() {
            h.append_raw(
                k.unwrap_or_else(|| "".parse().unwrap())
                    .as_str()
                    .to_string(),
                v.as_bytes().to_vec(),
            );
        }
        mime_multipart::FilePart::new(h, self.path.as_path())
    }
}

/// The extracted text fields and uploaded files from a `multipart/form-data` request.
///
/// Use `parse_multipart` to devise this object from a request.
///
/// Copyright © 2015 by Michael Dilger (of New Zealand)
///
/// This struct is licensed under the MIT license
#[derive(Clone, Debug, PartialEq)]
pub struct FormData {
    /// Name-value pairs for plain text fields. Technically, these are form data parts with no
    /// filename specified in the part's `Content-Disposition`.
    pub fields: Vec<(String, String)>,
    /// Name-value pairs for temporary files. Technically, these are form data parts with a filename
    /// specified in the part's `Content-Disposition`.
    pub files: Vec<(String, FilePart)>,
}

impl FormData {
    pub fn new() -> FormData {
        FormData {
            fields: vec![],
            files: vec![],
        }
    }

    /// Create a mime-multipart Vec<Node> from this FormData
    pub fn to_multipart(&self) -> Result<Vec<Node>, crate::Error> {
        use hyper::header::{
            ContentDisposition, ContentType, DispositionParam, DispositionType, Headers,
        };
        use mime::{Mime, SubLevel, TopLevel};
        // Translate to Nodes
        let mut nodes: Vec<Node> = Vec::with_capacity(self.fields.len() + self.files.len());

        for &(ref name, ref value) in &self.fields {
            let mut h = Headers::new();
            h.set(ContentType(Mime(TopLevel::Text, SubLevel::Plain, vec![])));
            h.set(ContentDisposition {
                disposition: DispositionType::Ext("form-data".to_owned()),
                parameters: vec![DispositionParam::Ext("name".to_owned(), name.clone())],
            });
            nodes.push(Node::Part(Part {
                headers: h,
                body: value.as_bytes().to_owned(),
            }));
        }

        for &(ref name, ref filepart) in &self.files {
            let mut filepart: mime_multipart::FilePart = filepart.clone().into();
            // We leave all headers that the caller specified, except that we rewrite
            // Content-Disposition.
            while filepart.headers.remove::<ContentDisposition>() {}
            let filename = match filepart.path.file_name() {
                Some(fname) => fname.to_string_lossy().into_owned(),
                None => return Err(crate::Error::InvalidFile),
            };
            filepart.headers.set(ContentDisposition {
                disposition: DispositionType::Ext("form-data".to_owned()),
                parameters: vec![
                    DispositionParam::Ext("name".to_owned(), name.clone()),
                    DispositionParam::Ext("filename".to_owned(), filename),
                ],
            });
            if filepart.headers.get::<ContentType>().is_none() {
                let guess = mime_guess::guess_mime_type(&filepart.path);
                filepart.headers.set(ContentType(guess));
            }
            nodes.push(Node::File(filepart));
        }

        Ok(nodes)
    }
    /// Turn into form stream
    pub fn into_form_stream(self) -> Result<FormStream<impl ConcatRead>, crate::Error> {
        let nodes = self.to_multipart()?;
        let boundary = generate_boundary();
        let (count, reader) = multipart_to_read(boundary.clone(), nodes)?;
        Ok(FormStream {
            boundary,
            reader,
            count,
        })
    }
}

/// Form stream with boundary, bytes count and a `std::io::Read`
#[derive(Debug)]
pub struct FormStream<T: ConcatRead + Send + Sync> {
    pub boundary: Vec<u8>,
    pub reader: T,
    pub count: usize,
}
