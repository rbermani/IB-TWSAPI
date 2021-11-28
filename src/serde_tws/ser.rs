use crate::serde_tws::error::*;
use crate::core::common::{UNSET_INTEGER, UNSET_INTEGER_I32_AS_I64, UNSET_INTEGER_I32_AS_U64, UNSET_DOUBLE};
use serde::{ser, Serialize};
use std::str;

pub struct Serializer {
    // This string begins empty; Fields are appended as values are serialized.
    output: String,
    payload_len: usize,
}

pub fn to_string<T: Serialize>(value: &T) -> Result<String> {
    let mut serializer = Serializer {
        output: "".to_owned(),
        payload_len: 0,
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        let mut out = "".to_owned();
        let val = (v as i32).to_string();

        if v >= UNSET_INTEGER_I32_AS_I64 {
            return self.serialize_i32(UNSET_INTEGER);
        }
        out.push_str(&val);
        self.serialize_str(&out)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        let mut out = "".to_owned();
        let val = (v as i32).to_string();

        if v >= UNSET_INTEGER_I32_AS_U64 {
            return self.serialize_i32(UNSET_INTEGER);
        }
        out.push_str(&val);
        self.serialize_str(&out)
    }
    fn serialize_i32(self, v: i32) -> Result<()> {
        let mut out = "".to_owned();
        let val = (v as i32).to_string();
        if UNSET_INTEGER != v {
           out.push_str(&val);
        }
        self.serialize_str(&out)
    }
    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        let mut out = "".to_owned();
        let val = v.to_string();
        if UNSET_DOUBLE != v {
            out.push_str(&val);
        }
        self.serialize_str(&out)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.output.push_str(&v);
        self.output.push_str("\u{0}");
        self.payload_len += v.len() + 1;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        Err(Error::Unsupported)
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
                println!("serialize_unit() not properly implemented!");

        self.output += "\0";

        self.payload_len += 1;
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        println!("serialize_unit_variantt()");

        self.serialize_str(&variant_index.to_string())
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        println!("serialize_newtype_struct()");

        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        println!("serialize_newtype_variant()");

        Err(Error::Unsupported)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::Unsupported)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::Unsupported)
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        println!("serialize_tuple_struct()");

        Err(Error::Unsupported)
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        println!("serialize_tuple_variant()");

        Err(Error::Unsupported)
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::Unsupported)
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        //println!("serialize_struct()");

        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        let term: &str = "\u{0}";
        let msg_id = (variant_index + 1).to_string() + term;
        self.payload_len = msg_id.len();
        self.output.push_str(&"\u{0}\u{0}\u{0}\u{0}");
        self.output.push_str(&msg_id);
        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported)
    }

    // Close the sequence.
    fn end(self) -> Result<()> {
        println!("SerializeSeq end()");

        Err(Error::Unsupported)
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        println!("SerializeTuple end()");

        Err(Error::Unsupported)
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        println!("SerializeMap end()");

        Err(Error::Unsupported)
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        println!("SerializeTupleStruct end()");

        Err(Error::Unsupported)
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported)
    }

    fn end(self) -> Result<()> {
        println!("SerializeTupleVariant end()");

        Err(Error::Unsupported)
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        //println!("SerializeStruct end()");

        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        //println!("SerializeStructVariant end()");

        let bytes = u32::to_be_bytes(self.payload_len as u32);
        let payload_len_bytes = str::from_utf8(&bytes).unwrap();
        self.output.replace_range(..4, payload_len_bytes);
        Ok(())
    }
}
////////////////////////////////////////////////////////////////////////////////
/*
#[test]
fn test_struct() {
    #[derive(Serialize)]
    struct Test {
        int: u32,
        seq: Vec<&'static str>,
    }

    let test = Test {
        int: 1,
        seq: vec!["a", "b"],
    };
    let expected = r#"{"int":1,"seq":["a","b"]}"#;
    assert_eq!(to_string(&test).unwrap(), expected);
}

#[test]
fn test_enum() {
    #[derive(Serialize)]
    enum E {
        Unit,
        Newtype(u32),
        Tuple(u32, u32),
        Struct { a: u32 },
    }

    let u = E::Unit;
    let expected = r#""Unit""#;
    assert_eq!(to_string(&u).unwrap(), expected);

    let n = E::Newtype(1);
    let expected = r#"{"Newtype":1}"#;
    assert_eq!(to_string(&n).unwrap(), expected);

    let t = E::Tuple(1, 2);
    let expected = r#"{"Tuple":[1,2]}"#;
    assert_eq!(to_string(&t).unwrap(), expected);

    let s = E::Struct { a: 1 };
    let expected = r#"{"Struct":{"a":1}}"#;
    assert_eq!(to_string(&s).unwrap(), expected);
}
*/
