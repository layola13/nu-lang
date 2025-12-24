// Nu Language AST Definitions
// 这些将用于Nu2Rust转换器

pub struct NuFile {
    pub items: Vec<NuItem>,
}

pub enum NuItem {
    Fn(NuFn),
    Struct(NuStruct),
    Enum(NuEnum),
    Trait(NuTrait),
    Impl(NuImpl),
    Use(NuUse),
    Mod(NuMod),
}

pub struct NuFn {
    pub name: String,
    pub is_public: bool,
    pub is_async: bool,
}

pub struct NuStruct {
    pub name: String,
    pub is_public: bool,
}

pub struct NuEnum {
    pub name: String,
    pub is_public: bool,
}

pub struct NuTrait {
    pub name: String,
    pub is_public: bool,
}

pub struct NuImpl {
    pub type_name: String,
}

pub struct NuUse {
    pub path: String,
    pub is_public: bool,
}

pub struct NuMod {
    pub name: String,
    pub is_public: bool,
}
