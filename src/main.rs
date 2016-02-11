extern crate encoding;
extern crate docopt;
extern crate shn;
extern crate xml;

use docopt::*;
use encoding::Encoding;
use xml::{EmitterConfig, EventWriter};
use xml::writer::XmlEvent;

use std::borrow::Cow;
use std::fs::File;
use std::io::{ Read, Write };
use std::path::Path;

const USAGE: &'static str = "
Usage: shn2xml [--encoding=<enc>] (--stdin | <input>) (--stdout | <output>)

Options:
    --encoding=<enc>        Sets the encoding
    -i, --stdin                 Sets input to be stdin
    -o --stdout                Sets output to stdout
";

fn main() {
    let args = Docopt::new(USAGE)
                        .and_then(|d| d.argv(std::env::args()).parse())
                        .unwrap_or_else(|e| e.exit());

    let input: Box<Read> = if args.get_bool("--stdin") {
        Box::new(std::io::stdin())
    } else {
        Box::new(open_file(args.get_str("<input>")))
    };
    let output: Box<Write> = if args.get_bool("--stdout") {
        Box::new(std::io::stdout())
    } else {
        Box::new(create_file(args.get_str("<output")))
    };
    let encoding = get_encoding(&args);

    let shn_file = match shn::ShnReader::read_from(input, &encoding) {
        Ok(f) => f,
        Err(e) => panic!("Could not read SHN file: {:?}", e),
    };

    write_to_xml(shn_file, output);
}

fn get_encoding(args: &docopt::ArgvMap) -> &'static encoding::types::EncodingRef {
    // encoding we use as default, if no option is given.
    const DEFAULT_ENCODING: &'static str = "ascii"; // TODO: Change to the windows stuff
    let mut encoding_name = args.get_str("--encoding");
    if encoding_name == "" {
        encoding_name = DEFAULT_ENCODING;
    };

    for enc in encoding::all::encodings() {
        if enc.name() == encoding_name {
            return enc;
        }
    }
    panic!("Encoding not found: '{}'", encoding_name);
}

fn open_file<P: AsRef<Path>>(path: P) -> File {
    match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => panic!("Could not open file: {}", e),
    }
}

fn create_file<P: AsRef<Path>>(path: P) -> File {
    match std::fs::OpenOptions::new().write(true).create(true).open(path) {
        Ok(f) => f,
        Err(e) => panic!("Could not create file: {}", e),
    }
}

fn write_to_xml<O: Write>(input: shn::ShnFile, output: O) {
    let mut writer: EventWriter<O> = EmitterConfig::new().perform_indent(true).create_writer(output);
    let start_doc_element = XmlEvent::StartElement {
        name: xml::name::Name::local("shnfile"),
        attributes: Cow::Owned(Vec::new()),
        namespace: Cow::Owned(xml::namespace::Namespace::empty())
    };

    // TODO: better error handling.
    writer.write(start_doc_element).ok();
    write_shn_schema_to_xml(&input, &mut writer);
    write_shn_data_to_xml(&input, &mut writer);
    writer.write(XmlEvent::EndElement { name: None }).ok();
}

fn write_shn_schema_to_xml<O: Write>(input: &shn::ShnFile, output: &mut EventWriter<O>) {
    output.write(XmlEvent::StartElement {
        name: xml::name::Name::local("schema"),
        attributes: Cow::Owned(vec![ xml::attribute::Attribute::new(
            xml::name::Name::local("cryptheader"),
            &bytes_to_string(&input.crypt_header[..]))]),
        namespace: Cow::Owned(xml::namespace::Namespace::empty())
    }).ok();

    for column in input.schema.columns.iter() {
        write_column(column, output);
    }

    output.write(XmlEvent::EndElement { name: None }).ok();
}

fn write_shn_data_to_xml<O: Write>(input: &shn::ShnFile, output: &mut EventWriter<O>) {
    for row in input.data.iter() {
        write_row_to_xml(row, input, output);
    }
}

fn write_row_to_xml<O: Write>(row: &shn::ShnRow, file: &shn::ShnFile, output: &mut EventWriter<O>) {
    let mut attrs = Vec::with_capacity(row.data.len());
    for i in 0..file.schema.columns.len() {
        let col = file.schema.columns.get(i).unwrap();
        let cell = row.data.get(i).unwrap();
        let cell_value = cell_to_str(cell);
        attrs.push(xml::attribute::OwnedAttribute::new(
            xml::name::OwnedName::local(col.name.to_string()),
            cell_value));
    }
    let attrs: Vec<xml::attribute::Attribute> = attrs.iter().map(xml::attribute::OwnedAttribute::borrow).collect();

    output.write(XmlEvent::StartElement {
        name: xml::name::Name::local("row"),
        attributes: Cow::Owned(attrs),
        namespace: Cow::Owned(xml::namespace::Namespace::empty())
    }).unwrap();
    output.write(XmlEvent::EndElement { name: None }).unwrap();
}

fn write_column<O: Write>(column: &shn::ShnColumn, output: &mut EventWriter<O>) {
    let attributes = vec![
        xml::attribute::Attribute::new(xml::name::Name::local("name"), &column.name),
        xml::attribute::Attribute::new(xml::name::Name::local("type"), type_to_str(&column.data_type))
    ];

    output.write(XmlEvent::StartElement {
        name: xml::name::Name::local("shncolumn"),
        attributes: Cow::Owned(attributes),
        namespace: Cow::Owned(xml::namespace::Namespace::empty())
    }).ok();
    output.write(XmlEvent::EndElement { name: None }).ok();
}

fn type_to_str(t: &shn::ShnDataType) -> &'static str {
    match t {
        &shn::ShnDataType::StringFixedLen => "StringFL",
        &shn::ShnDataType::StringZeroTerminated => "StringSZ",
        &shn::ShnDataType::Byte => "u8",
        &shn::ShnDataType::SignedByte => "i8",
        &shn::ShnDataType::UnsignedShort => "u16",
        &shn::ShnDataType::SignedShort => "i16",
        &shn::ShnDataType::UnsignedInteger => "u32",
        &shn::ShnDataType::SignedInteger => "i32",
        &shn::ShnDataType::SingleFloatingPoint => "f32",
    }
}

fn bytes_to_string(bytes: &[u8]) -> String {
    let mut s = String::with_capacity((bytes.len() * 3) - 1);
    for i in 0..bytes.len() {
        if i > 0 {
            s.push(' ');
        };
        s.push_str(&(format!("{:x}", bytes[i])));
    };
    s
}

fn cell_to_str(cell: &shn::ShnCell) -> String {
    match cell {
        &shn::ShnCell::StringFixedLen(ref s) => s.to_string(),
        &shn::ShnCell::StringZeroTerminated(ref s) => s.to_string(),
        &shn::ShnCell::Byte(ref b) => b.to_string(),
        &shn::ShnCell::SignedByte(ref b) => b.to_string(),
        &shn::ShnCell::UnsignedShort(ref s) => s.to_string(),
        &shn::ShnCell::SignedShort(ref s) => s.to_string(),
        &shn::ShnCell::UnsignedInteger(ref i) => i.to_string(),
        &shn::ShnCell::SignedInteger(ref i) => i.to_string(),
        &shn::ShnCell::SingleFloatingPoint(ref f) => f.to_string()
    }
}
