use std::collections::HashMap;
use fnv::FnvHashMap;
use http::StatusCode;
use crate::global::common::rpc::RpcResult;
use crate::global::db::core::DBConn;
use crate::global::rest::db::query_context::{get_parent_node_path, QueryContext, QueryNode, RATIO_PRIMARY};
use crate::global::rest::db::query_executor::DEFAULT_MAX_COUNT;
use crate::global::rest::utils::transform::transform_salve_value;

/// 处理GET请求的异步方法
///
/// # 参数
/// * `body_map` - 包含请求参数的HashMap，键为String类型，值为serde_json::Value类型
///
/// # 返回值
/// 返回serde_json::Value类型的JSON响应数据
pub async fn handle_get(db: &DBConn, body_map: HashMap<String, serde_json::Value>) -> RpcResult::<HashMap<String, serde_json::Value>> {
    let mut ctx = QueryContext::from_json(body_map);
    ctx.response(db).await
}

impl QueryContext {
    async fn response(&mut self, db: &DBConn) -> RpcResult::<HashMap<String, serde_json::Value>> {
        // 克隆 query_node 以避免借用冲突
        let query_node = self.layer_query_node.clone();

        // 按层级遍历节点并处理
        for nodes in query_node.values() {
            // 按权重降序排序
            let mut sorted_nodes = nodes.clone();
            sorted_nodes.sort_unstable_by(|a, b| b.borrow().weight.cmp(&a.borrow().weight));
            for node in sorted_nodes {
                let mut node_rc = node.borrow_mut();
                if node_rc.weight >= RATIO_PRIMARY {
                    self.query_primary_node(&mut *node_rc, db).await;
                } else {
                    self.query_relate_node(&mut *node_rc, db).await;
                }
            }
        }

        // 构建响应结果映射
        let mut response_payload = HashMap::new();
        // 遍历所有主节点数据（每个主节点路径及其对应的查询结果）
        for (node_path, results) in &self.primary_node_data {
            // 获取当前主节点的引用
            let node_ref = self.query_node.get(node_path).unwrap().borrow();
            // 获取命名空间（父节点路径）
            let namespace = get_parent_node_path(node_path);
            // 获取主节点名称
            let node_name = &node_ref.name;
            // 判断主节点是否为列表类型
            let is_list = node_ref.is_list;
            // 获取主节点与从节点的关联字段映射关系
            let primary_relate_kv = self.primary_relate_kv.get(node_path).cloned().unwrap_or_default();

            if is_list {
                // 如果主节点是列表类型，遍历每个结果，构建主节点及其关联从节点的嵌套结构
                let primary_node_result_list: Vec<_> = results.iter()
                    .map(|result| self.build_primary_value(&namespace, node_name, result, &primary_relate_kv))
                    .collect();
                // 将结果列表插入到响应映射中，键为命名空间
                response_payload.insert(namespace, serde_json::json!(primary_node_result_list));
            } else {
                // 如果主节点不是列表类型，取第一个结果（若无则用默认值）
                let result = results.first().cloned().unwrap_or_default();
                // 构建主节点及其关联从节点的嵌套结构
                let primary_value = self.build_primary_value(&namespace, node_name, &result, &primary_relate_kv);
                // 将主节点及其关联从节点的所有键值对插入到响应映射中
                for (key, value) in primary_value {
                    response_payload.insert(key, value);
                }
            }
        }

        let status_code = self.code;
        let err_msg = &self.err_msg;
        RpcResult::<HashMap<String, serde_json::Value>>{ code: status_code, msg: err_msg.to_owned(), payload: Some(response_payload) }
    }

    fn build_primary_value(&self, namespace: &str, primary_node_name: &str, primary_node_data: &HashMap<String, serde_json::Value>, primary_relate_kv: &HashMap<String, String>) -> HashMap<String, serde_json::Value> {
        let mut result_map = HashMap::<String, serde_json::Value>::new();

        // 主节点数据
        result_map.insert(primary_node_name.to_string(), serde_json::to_value(primary_node_data.clone()).unwrap());

        // 从节点数据
        for (primary_field, slave_node_field_path) in primary_relate_kv {
            // 获取主节点中关联字段的值，用于查询从节点数据
            let primary_field_value = primary_node_data.get(primary_field).unwrap();
            // 从路径中提取从节点字段名称（取最后一个斜杠后的部分）
            let slave_node_field = slave_node_field_path.split("/").last().unwrap();
            // 构建从节点字段值的键，格式为"字段名/字段值"
            let slave_node_field_value_key = format!("{}/{}", slave_node_field, primary_field_value);
            // 根据从节点字段路径和值键获取对应的从节点数据
            let slave_node_field_data_opt = self.get_slave_node_data(slave_node_field_path, &slave_node_field_value_key);
            if slave_node_field_data_opt.is_some() {
                // 计算从节点数据的相对路径
                // 1. 首先获取从节点字段路径的父路径
                // 2. 尝试去除命名空间前缀
                // 3. 如果去除成功且结果为空，则使用原始父路径
                // 4. 如果去除失败，也使用原始父路径
                let node_data_relative_path = get_parent_node_path(slave_node_field_path)
                    .strip_prefix(&format!("{}/", namespace))
                    .map_or_else(
                        || get_parent_node_path(slave_node_field_path),  // 前缀去除失败时的处理
                        |stripped| if stripped.is_empty() {  // 前缀去除成功但结果为空的处理
                            get_parent_node_path(slave_node_field_path) 
                        } else {  // 前缀去除成功且结果非空的处理
                            stripped.to_string() 
                        }
                    );
                
                // 根据相对路径的格式决定如何处理从节点数据
                if node_data_relative_path.contains("/") {
                    // 如果路径包含"/"，表示需要进行嵌套结构转换
                    if let Some(slave_data) = slave_node_field_data_opt {
                        // 创建一个只有一个键值对的映射，用于转换
                        let slave_field_value_map = std::iter::once((node_data_relative_path, slave_data)).collect::<HashMap<_, _>>();
                        // 使用transform_salve_value函数将扁平结构转换为嵌套结构
                        result_map.extend(transform_salve_value(slave_field_value_map));
                    }
                } else {
                    // 如果路径不包含"/"，直接将数据添加到结果映射中
                    result_map.insert(node_data_relative_path, slave_node_field_data_opt.unwrap());
                }
            }
        }
        
        result_map
    }

    /// 获取从节点关联数据
    ///
    /// # 参数
    /// * `slave_node_field_path` - 从节点字段路径，格式为"命名空间/字段名"
    /// * `slave_node_field_value_key` - 从节点字段值键，格式为"字段名/字段值"
    ///
    /// # 返回值
    /// 返回Option<serde_json::Value>，包含从节点数据的JSON值或None
    fn get_slave_node_data(&self, slave_node_field_path: &str, slave_node_field_value_key: &str) -> Option<serde_json::Value> {
        // 获取从节点路径（去除字段名部分）
        let slave_node_path = get_parent_node_path(slave_node_field_path);
        
        // 链式操作获取从节点数据：
        // 1. 首先从slave_node_relate_data获取路径对应的字段映射表
        // 2. 然后从字段映射表获取指定键的值
        // 3. 最后处理获取到的关联数据
        self.slave_node_relate_data.get(&slave_node_path)
            .and_then(|field_map| field_map.get(slave_node_field_value_key))
            .and_then(|relate_field_data| {
                // 检查从节点是否为列表类型
                let is_list = self.query_node.get(&slave_node_path)?.borrow().is_list;
                
                if relate_field_data.is_empty() { // 记录空数据日志
                    log::debug!("slave.data: {}.{} is empty", &slave_node_path, slave_node_field_value_key);
                    None
                } else if is_list { // 列表类型：直接序列化整个数组
                    Some(serde_json::to_value(relate_field_data).unwrap())
                } else { // 非列表类型：只取第一个元素序列化
                    Some(serde_json::to_value(&relate_field_data[0]).unwrap())
                }
            }
        )
    }

    async fn query_primary_node(&mut self, node: &mut QueryNode, db: &DBConn) {
        // 添加关联字段到查询列
        if let Some(primary_relate_kv) = self.primary_relate_kv.get(&node.path) {
            for (column, _) in primary_relate_kv {
                node.sql_executor.add_column(column);
            }
        }

        // 查询主节点数据
        let Some(node_results) = self.query_node_data(node, db).await else { return };

        // 保存查询结果
        self.primary_node_data.insert(node.path.clone(), node_results.clone());

        // 处理查询结果
        if node.is_list {
            self.process_list_results(node, node_results);
        } else if let Some(result) = node_results.first() {
            self.process_single_result(node, result);
        }
    }

    // 处理列表类型结果
    fn process_list_results(&mut self, node: &QueryNode, results: Vec<HashMap<String, serde_json::Value>>) {
        for result in results {
            for (k, v) in result {
                let full_path = format!("{}/{}", node.path, k);
                if let Some(entry) = self.primary_node_related_field_values.get_mut(&full_path) {
                    match entry {
                        serde_json::Value::Null => *entry = serde_json::Value::Array(vec![v]),
                        serde_json::Value::Array(arr) => arr.push(v),
                        _ => {}
                    }
                }
            }
        }
    }

    // 处理单个结果
    fn process_single_result(&mut self, node: &QueryNode, result: &HashMap<String, serde_json::Value>) {
        for (k, v) in result {
            let full_path = format!("{}/{}", node.path, k);
            if let Some(existing_value_slot) = self.primary_node_related_field_values.get_mut(&full_path) {
                *existing_value_slot = v.clone();
            }
        }
    }

    async fn query_relate_node(&mut self, node: &mut QueryNode, db: &DBConn) {
        // 获取当前节点的路径和关联字段映射关系
        let node_path = node.path.clone();
        let node_relate_kv = self.slave_relate_kv.get(&node_path).cloned().unwrap_or_default();

        // 处理每个关联字段的查询条件
        for (field_name, primary_node_field_path) in &node_relate_kv {
            // 从主节点获取关联字段的值
            if let Some(value) = self.primary_node_related_field_values.get(primary_node_field_path) {
                // 如果值为空则直接返回
                if value.is_null() {
                    return;
                }
                // 如果是数组类型，设置分页大小为数组长度
                if let serde_json::Value::Array(array) = value {
                    node.sql_executor.page_size(serde_json::json!(0), serde_json::json!(array.len()));
                }
                // 解析查询条件
                node.sql_executor.parse_condition(field_name, value);
            } else {
                continue;
            }
            // 确保关联字段在查询字段列表中
            node.sql_executor.add_column(field_name);
        }

        // 执行节点数据查询
        if let Some(node_results) = self.query_node_data(node, db).await {
            // 处理每个关联字段的查询结果
            for (field, _) in &node_relate_kv {
                let mut field_map = FnvHashMap::<String, Vec<HashMap<String, serde_json::Value>>>::default();
                // 处理字段名后缀@的情况
                let field_key = if field.ends_with('@') {
                    &field[..field.len() - 1]
                } else {
                    field.as_str()
                };
                // 遍历查询结果，构建字段映射关系
                for result in &node_results {
                    if let Some(field_value) = result.get(field_key) {
                        // 构建字段路径格式：字段名/字段值
                        let field_path = format!("{}/{}", field_key, field_value);
                        // 将结果存入字段映射表
                        field_map.entry(field_path).or_insert_with(Vec::new).push(result.clone());
                    }
                }
                // 将字段映射表存入从节点关联数据
                self.slave_node_relate_data.insert(node.path.clone(), field_map);
            }
        }
    }

    async fn query_node_data(&mut self, node: &mut QueryNode, db: &DBConn) -> Option<Vec<HashMap<String, serde_json::Value>>> {
        // 准备SQL查询的基本参数
        let node_name = &node.name.to_lowercase();
        let node_path = &node.path;
        let node_attrs = &node.attributes;
        // 设置查询的表名
        let _ = node.sql_executor.parse_table(node_name);
        // 解析节点属性中的查询条件
        for (key, value) in node_attrs {
            let _ = node.sql_executor.parse_condition(key, value);
        }
        
        // 处理列表查询的分页逻辑
        if node.is_list {
            let parent_path = get_parent_node_path(node_path);
            // 尝试从父节点获取分页参数
            if let Some(parent_node_attrs) = self.namespace_node.get(&parent_path).cloned() {
                // 获取页码和每页数量，如果不存在则使用默认值
                let page = parent_node_attrs.get("page").cloned().unwrap_or_else(|| serde_json::json!(0));
                let count = parent_node_attrs.get("count").cloned().unwrap_or_else(|| serde_json::json!(DEFAULT_MAX_COUNT));
                node.sql_executor.page_size(page, count);
            } else { // 父节点不存在时使用默认分页参数
                node.sql_executor.page_size(serde_json::json!(0), serde_json::json!(DEFAULT_MAX_COUNT));
            }
        }
        
        // 执行SQL查询并处理结果
        match node.sql_executor.exec(db).await {
            Ok(results) => { Some(results) },
            Err(e) => { // 保存错误信息到上下文
                self.err_msg = Some(e.to_string());
                self.code = StatusCode::INTERNAL_SERVER_ERROR;
                None
            }
        }
    }
}