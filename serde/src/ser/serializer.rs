use super::escape_string::{escape_str, need_quotes};
use crate::error::SerializeError;
use serde::{Serialize, ser};

pub struct Serializer {
    output: String,
    indent: i32,
}

pub type Result<T> = std::result::Result<T, SerializeError>;

pub fn to_string<T: Serialize>(value: &T) -> Result<String> {
    let mut serializer = Serializer {
        output: String::new(),
        indent: 0,
    };
    value.serialize(&mut serializer)?;
    serializer.end_file();
    Ok(serializer.output)
}

impl Serializer {
    fn newline_with_indent(&mut self) {
        if !self.output.is_empty() {
            // have to do this check to avoid an initial new line for top
            // level mappings and sequences
            self.output += "\n";
            self.output
                .push_str(&(" ".to_string().repeat(self.indent as usize * 2)));
        }
    }

    fn increase_indent(&mut self) {
        self.indent += 1;
    }

    fn decrease_indent(&mut self) {
        self.indent -= 1;
        assert!(self.indent >= 0);
    }

    fn end_file(&mut self) {
        self.output += "\n";
    }

    fn prepend_space_if_value(&mut self) {
        if self.output.ends_with(':') {
            self.output += " ";
        }
    }

    fn append_as_string_value<T: ToString>(&mut self, v: &T) -> Result<()> {
        self.prepend_space_if_value();
        self.output += &v.to_string();
        Ok(())
    }
}

impl ser::Serializer for &mut Serializer {
    type Ok = ();

    type Error = SerializeError;

    type SerializeSeq = Self;

    type SerializeTuple = Self;

    type SerializeTupleStruct = Self;

    type SerializeTupleVariant = Self;

    type SerializeMap = Self;

    type SerializeStruct = Self;

    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> std::result::Result<Self::Ok, Self::Error> {
        self.append_as_string_value(&v)
    }

    fn serialize_i8(self, v: i8) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> std::result::Result<Self::Ok, Self::Error> {
        self.append_as_string_value(&v)
    }

    fn serialize_u8(self, v: u8) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> std::result::Result<Self::Ok, Self::Error> {
        self.append_as_string_value(&v)
    }

    fn serialize_f32(self, v: f32) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> std::result::Result<Self::Ok, Self::Error> {
        self.append_as_string_value(&v)
    }

    fn serialize_char(self, v: char) -> std::result::Result<Self::Ok, Self::Error> {
        self.append_as_string_value(&v)
    }

    fn serialize_str(self, v: &str) -> std::result::Result<Self::Ok, Self::Error> {
        self.prepend_space_if_value();

        if v.contains("\n") {
            self.output += "|";
            for line in v.lines() {
                self.newline_with_indent();
                self.output += line;
            }
        } else if need_quotes(v) {
            escape_str(&mut self.output, v)?;
        } else {
            self.output += v;
        }
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> std::result::Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_none(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.prepend_space_if_value();
        self.output += "null";
        Ok(())
    }

    fn serialize_unit_struct(
        self,
        _name: &'static str,
    ) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_str(_variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        variant.serialize(&mut *self)?;
        self.output += ":";
        self.increase_indent();
        value.serialize(&mut *self)?;
        self.decrease_indent();
        Ok(())
    }

    fn serialize_seq(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> std::result::Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleVariant, Self::Error> {
        variant.serialize(&mut *self)?;
        self.output += ":";
        self.increase_indent();
        Ok(self)
    }

    fn serialize_map(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeMap, Self::Error> {
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(_len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStructVariant, Self::Error> {
        variant.serialize(&mut *self)?;
        self.output += ":";
        self.increase_indent();
        Ok(self)
    }
}

impl ser::SerializeSeq for &mut Serializer {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.newline_with_indent();
        self.output += "- ";
        self.increase_indent();
        value.serialize(&mut **self)?;
        self.decrease_indent();
        Ok(())
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeTuple for &mut Serializer {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.newline_with_indent();
        self.output += "- ";
        self.increase_indent();
        value.serialize(&mut **self)?;
        self.decrease_indent();
        Ok(())
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut Serializer {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_field<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.newline_with_indent();
        self.output += "- ";
        self.increase_indent();
        value.serialize(&mut **self)?;
        self.decrease_indent();
        Ok(())
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
impl ser::SerializeTupleVariant for &mut Serializer {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_field<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.newline_with_indent();
        self.output += "- ";
        self.increase_indent();
        value.serialize(&mut **self)?;
        self.decrease_indent();
        Ok(())
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.decrease_indent();
        Ok(())
    }
}
impl ser::SerializeMap for &mut Serializer {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_key<T>(&mut self, key: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.newline_with_indent();
        key.serialize(&mut **self)?;
        self.output += ":";
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.increase_indent();
        value.serialize(&mut **self)?;
        self.decrease_indent();
        Ok(())
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeStruct for &mut Serializer {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.newline_with_indent();
        key.serialize(&mut **self)?;
        self.output += ":";
        self.increase_indent();
        value.serialize(&mut **self)?;
        self.decrease_indent();
        Ok(())
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut Serializer {
    type Ok = ();
    type Error = SerializeError;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.newline_with_indent();
        key.serialize(&mut **self)?;
        self.output += ":";
        self.increase_indent();
        value.serialize(&mut **self)?;
        self.decrease_indent();
        Ok(())
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.decrease_indent();
        Ok(())
    }
}
