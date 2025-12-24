// TypeScript转换器配置类型

#[derive(Clone)]
pub struct TsConfig {
    /// 运行时注入模式: inline 或 import
    pub runtime_mode: RuntimeMode,
    /// 目标平台: node, browser, deno
    pub target: Target,
    /// 严格模式：不允许的std库调用会报错
    pub strict: bool,
    /// 禁用 $fmt，使用字符串模板
    pub no_format: bool,
    /// 生成 source map
    pub source_map: bool,
}

#[derive(Clone, PartialEq)]
pub enum RuntimeMode {
    Inline,  // 注入到每个文件
    Import,  // 外部模块引用
}

#[derive(Clone, PartialEq)]
pub enum Target {
    Node,
    Browser,
    Deno,
}

impl Default for TsConfig {
    fn default() -> Self {
        Self {
            runtime_mode: RuntimeMode::Inline,
            target: Target::Node,
            strict: true,
            no_format: false,
            source_map: false,
        }
    }
}

#[derive(Clone)]
pub(crate) struct StructInfo {
    pub name: String,
    pub fields: Vec<String>,
    pub is_public: bool,
}

#[derive(Clone)]
pub(crate) struct EnumVariant {
    pub name: String,
    pub data: Option<String>,
}

#[derive(Clone)]
pub(crate) struct EnumInfo {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub is_public: bool,
}

#[derive(Clone)]
pub(crate) struct ImplInfo {
    pub target: String,
    pub methods: Vec<String>,
}

#[derive(Default)]
pub(crate) struct ConversionContext {
    pub in_trait: bool,
    pub in_impl: bool,
    pub in_enum_impl: bool,  // 新增：标记是否在enum的impl中（生成namespace）
    pub in_function: bool,
    pub temp_var_counter: usize,
    pub current_class: Option<String>,
    // 状态跟踪：用于合并struct和impl
    pub structs: std::collections::HashMap<String, StructInfo>,
    pub enums: std::collections::HashMap<String, EnumInfo>,
    pub impls: std::collections::HashMap<String, Vec<ImplInfo>>,
    // 当前正在处理的块
    pub current_struct: Option<String>,
    pub current_enum: Option<String>,
    pub current_impl: Option<String>,
    pub current_impl_enum: Option<String>,  // 新增：当前enum impl的名称
}

impl ConversionContext {
    pub fn next_temp_var(&mut self) -> String {
        let var = format!("_tmp{}", self.temp_var_counter);
        self.temp_var_counter += 1;
        var
    }
    
    pub fn reset_temp_counter(&mut self) {
        self.temp_var_counter = 0;
    }
}