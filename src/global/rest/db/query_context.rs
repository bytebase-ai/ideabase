use fnv::FnvHashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use http::status::StatusCode;
use crate::global::rest::db::query_executor::QueryExecutor;

/// 主节点权重常量
pub const RATIO_PRIMARY: i32 = 10000;
/// 被依赖一次的权重系数
const RATIO_RELATED: i32 = 10;


#[derive(Debug)]
pub struct QueryContext {
    // 状态码
    pub code: StatusCode,
    // 错误信息
    pub err_msg: Option<String>,

    // 主节点字段映射表(主节点路径 -> 主节点字段 -> 指向从节点关联字段路径)
    pub primary_relate_kv: FnvHashMap<String, HashMap<String, String>>,
    // 从节点字段映射表(从节点路径 -> 从节点字段 -> 指向主节点关联字段路径)
    pub slave_relate_kv: FnvHashMap<String, HashMap<String, String>>,

    // 分层节点，层级: 节点列表
    pub layer_query_node: BTreeMap<i32, Vec<Rc<RefCell<QueryNode>>>>,
    // 命名空间节点
    pub namespace_node: FnvHashMap<String, FnvHashMap<String, serde_json::Value>>,
    // 数据查询节点，节点路径: 节点
    pub query_node: FnvHashMap<String, Rc<RefCell<QueryNode>>>,

    // 主节点数据列表(节点路径 -> 结果数据)，主节点就是每一个命名空间的主查询节点
    pub primary_node_data: FnvHashMap<String, Vec<HashMap<String, serde_json::Value>>>,
    // 被关联字段的值(主节点字段路径 -> 主节点字段值(默认Value::Null, 结果是array or object))
    pub primary_node_related_field_values: FnvHashMap<String, serde_json::Value>,
    // 从节点关联字段映射表(从节点父路径 -> 字段对应的值), 用于主节点获取从节点数据
    pub slave_node_relate_data: FnvHashMap<String, FnvHashMap<String, Vec<HashMap<String, serde_json::Value>>>>,
}

#[derive(Debug)]
pub struct QueryNode {
    // 节点名称
    pub name: String,
    // 当前节点的完整路径
    pub path: String,
    // 标记是否是列表查询
    pub is_list: bool,
    /// 属性映射
    pub attributes: HashMap<String, serde_json::Value>,
    // 权重
    pub weight: i32,
    // SQL执行器，负责生成和执行SQL
    pub sql_executor: QueryExecutor,
}


impl QueryContext {
    /// 从 JSON 值构建 QueryContext
    pub fn from_json(root: HashMap<String, serde_json::Value>) -> Self {
        // 创建处理队列，每项包含：(父路径, 节点名称, 节点值, 深度)
        let mut json_vec_deque: VecDeque<(String, String, serde_json::Value, i32)> = VecDeque::new();

        // 分层节点，层级: 节点列表
        let mut layer_query_node: BTreeMap<i32, Vec<Rc<RefCell<QueryNode>>>> = BTreeMap::default();
        // 初始化数据结构，用于构建查询上下文
        let mut namespace_node = FnvHashMap::default();
        // 数据查询节点，节点路径: 节点
        let mut query_node: FnvHashMap<String, Rc<RefCell<QueryNode>>> = FnvHashMap::default();

        // 主节点字段映射表(主节点路径 -> 主节点字段 -> 指向从节点关联字段路径)
        let mut primary_relate_kv: FnvHashMap<String, HashMap<String, String>> = FnvHashMap::default();
        // 从节点字段映射表(从节点路径 -> 从节点字段 -> 指向主节点关联字段路径)
        let mut slave_relate_kv: FnvHashMap<String, HashMap<String, String>> = FnvHashMap::default();

        // 被关联字段的值(主节点字段路径 -> 主节点字段值(默认Value::Null, 结果是array or object))
        let mut primary_node_related_field_values: FnvHashMap<String, serde_json::Value> = FnvHashMap::default();

        // 处理根节点，区分数组节点和普通节点
        for (key, val) in root {
            if key.ends_with("[]") {
                // 处理数组节点：收集标量属性并将子对象加入队列
                namespace_node.insert(key.clone(), collect_scalar_attrs(&val));
                // 检查当前值是否为JSON对象
                if let Some(map) = val.as_object() {
                    // 遍历对象的所有键值对
                    map.iter()
                        // 过滤出值也是对象的键值对
                        .filter(|(_, v)| v.is_object())
                        // 对每个符合条件的键值对执行操作
                        .for_each(|(k, v)| {
                            // 将子对象加入处理队列：
                            // - 父路径: 当前数组节点的key
                            // - 子节点名称: k
                            // - 子节点值: v 
                            // - 深度设为2(相对于根节点的深度)
                            json_vec_deque.push_back((key.clone(), k.clone(), v.clone(), 2));
                        }
                    );
                }
            } else { // 处理普通节点：直接加入队列，深度为1
                if let Some(_) = val.as_object() {
                    json_vec_deque.push_back((String::new(), key.clone(), val.clone(), 1));
                }
            }
        }

        // 从队列中依次处理节点，构建查询上下文
        while let Some((parent_path, name, node_val, depth)) = json_vec_deque.pop_front() {
            // 构建节点的完整路径
            let node_path = if parent_path.is_empty() { name.clone() } else { format!("{}/{}", parent_path, name) };

            // 处理数组节点（名称以[]结尾的节点）
            if name.ends_with("[]") {
                namespace_node.insert(node_path.clone(), collect_scalar_attrs(&node_val));
                // 递归处理数组节点的子对象
                if let Some(map) = node_val.as_object() {
                    map.iter()
                        .filter(|(_, v)| v.is_object())
                        .for_each(|(k, v)| {
                            json_vec_deque.push_back((node_path.clone(), k.clone(), v.clone(), depth + 1));
                        }
                    );
                }
            } else { // 处理普通节点，提取属性和关联关系
                let mut attributes = HashMap::new();
                // 判断节点是否属于列表（父节点是否为数组）
                let mut is_list = parent_path.ends_with("[]");
                if let Some(map) = node_val.as_object() {
                    for (field_key, field_value) in map {
                        if is_scalar_field(field_value) {
                            // 解析字符串值中的路径引用，建立节点间的关联关系
                            if field_key.ends_with('@') {
                                let field_name = field_key[..(field_key.len()-1)].to_string();
                                let field_path = format!("{}/{}", &node_path, field_name);
                                 // 依赖关系是唯一索引则节点数据结果一定不是 list
                                 if field_name.as_str() == "id" { is_list = false; }
                                // 关联关系
                                if let serde_json::Value::String(primary_field_path) = field_value {
                                    slave_relate_kv.entry(node_path.clone()).or_default().insert(field_name, primary_field_path.to_string());
                                    // 添加主节点字段对应的值到值映射表中
                                    primary_node_related_field_values.insert(primary_field_path.to_string(), serde_json::Value::Null);
                                    // 添加主节点字段对应的值到字段映射表中
                                    let index = primary_field_path.rfind('/').unwrap_or(0);
                                    let primary_node_path = &primary_field_path[..index];
                                    let primary_related_field = &primary_field_path[(index+1)..];
                                    primary_relate_kv.entry(primary_node_path.to_string()).or_default().insert(primary_related_field.to_string(), field_path.to_string());
                                }
                            } else { // 普通查询属性
                                attributes.insert(field_key.clone(), field_value.clone());
                            }
                        }
                    }
                }
                
                // 创建查询节点并添加到对应深度的节点列表中
                let shared_node = Rc::new(RefCell::new(QueryNode {
                    name: (&name).to_string(),
                    path: node_path.clone(),
                    weight: 0,
                    is_list,
                    attributes,
                    sql_executor: QueryExecutor::new(),
                }));
                layer_query_node.entry(depth).or_default().push(shared_node.clone());
                query_node.insert(node_path, shared_node);
            }
        }

        let mut ctx = QueryContext { code: StatusCode::OK, err_msg: None,
            layer_query_node,
            namespace_node,
            query_node,

            primary_relate_kv,
            slave_relate_kv,

            primary_node_related_field_values,
            slave_node_relate_data: FnvHashMap::default(),
            primary_node_data: FnvHashMap::default(),
        };
        ctx.compute_node_weight();
        ctx
    }

    /// 计算每个节点的权重
    fn compute_node_weight(&mut self) {
        // 收集所有节点引用，将多层嵌套的节点扁平化为一个列表
        let all_nodes: Vec<Rc<RefCell<QueryNode>>> = self.layer_query_node.values().flatten().cloned().collect();

        // 统计每个节点被依赖的次数，用于后续权重计算
        let mut counts: HashMap<String, u32> = HashMap::new();
        for node_rc in &all_nodes {
            let node_path = node_rc.borrow().path.clone();
            // 检查当前节点是否有从属关系映射
            if let Some(relate) = self.slave_relate_kv.get(&node_path) {
                // 遍历所有从属关系映射值（主节点路径）
                relate.values().for_each(|parent_path| {
                    // 更新主节点被依赖计数：
                    // 1. 如果主节点路径不存在于counts中，则插入并初始化为0
                    // 2. 对主节点路径的计数加1
                    *counts.entry(parent_path.clone()).or_insert(0) += 1;
                });
            }
        }

        // 根据节点的依赖关系计算权重
        // 1. 无依赖的节点获得基础权重 RATIO_PRIMARY
        // 2. 被依赖的节点获得额外权重 RATIO_RELATED^count
        for node_rc in &all_nodes {
            let (path, has_dep) = {
                let b = node_rc.borrow();
                let b_path = b.path.clone();
                let b_relate_kv = self.slave_relate_kv.get(&b_path);
                (b_path, b_relate_kv.is_some())
            };
            let count = counts.get(&path).copied().unwrap_or(0);
            let addition = if count > 0 { RATIO_RELATED.pow(count) } else { 0 };
            let weight = if !has_dep { RATIO_PRIMARY + addition } else { addition };
            node_rc.borrow_mut().weight = weight;
        }
    }
}

/// 判断是否为标量
fn is_scalar_field(v: &serde_json::Value) -> bool { v.is_number() || v.is_string() || v.is_boolean() }

/// 从 JSON 对象中收集标量属性
fn collect_scalar_attrs(v: &serde_json::Value) -> FnvHashMap<String, serde_json::Value> {
    // 从JSON值中收集标量属性（数字、字符串、布尔值）
    match v.as_object() {
        // 如果值是JSON对象
        Some(obj) => obj.iter()
            // 过滤出标量字段（非对象/数组）
            .filter(|(_, val)| is_scalar_field(val))
            // 将键值对转换为(String, Value)元组
            .map(|(k, val)| (k.clone(), val.clone())).collect(),
        // 如果不是JSON对象，返回空哈希表
        None => FnvHashMap::default(),
    }
}

// 获取父节点路径
// 参数: node_path - 当前节点的完整路径字符串
// 返回值: 父节点路径字符串，如果没有父节点则返回空字符串
pub fn get_parent_node_path(node_path: &str) -> String {
    node_path.rsplit_once('/').map(|(parent, _)| parent.to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::global::common::json::parse_to_json_value;
    use crate::global::common::utils::serde_json_map_to_hashmap;
    use crate::global::rest::db::query_context::QueryContext;

    #[test]
    fn test_query_ctx() {
        let json_str = r#"
    {
      "[]":{
        "page":0,
        "count":2,
        "timeline.Moment":{ "content$":"%a%" },
        "timeline.User":{ "id@":"[]/timeline.Moment/user_id","@column":"id,username,avatar" },
        "Comment[]":{
          "count":2,
          "timeline.Comment":{"moment_id@":"[]/timeline.Moment/id"},
          "timeline.User":{ "id@":"[]/Comment[]/timeline.Comment/user_id","@column":"id,username,avatar" }
        }
      },
      "timeline.Moment":{ "id":28710 },
      "timeline.User":{ "id@":"timeline.Moment/user_id" }
    }
    "#;

        let v = parse_to_json_value(json_str);
        let ctx = QueryContext::from_json(serde_json_map_to_hashmap(v.as_object().unwrap()));
        for (k, v) in &ctx.layer_query_node {
            println!("{}: {:?}", k, v);
        }
    }
}
