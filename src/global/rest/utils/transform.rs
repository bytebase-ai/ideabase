use std::collections::HashMap;

/// 将一个平铺了 "a/b/c" 风格键的 JSON 对象，转换为嵌套版本
pub fn transform_salve_value(input_map: HashMap<String, serde_json::Value>) -> HashMap<String, serde_json::Value> {
    let mut result_map = HashMap::new();
    
    for (raw_key, value) in input_map {
        // 拆分路径
        let segments: Vec<&str> = raw_key.split('/').collect();
        
        // 如果没有嵌套路径，直接插入
        if segments.len() == 1 {
            result_map.insert(raw_key, value);
            continue;
        }
        
        let first = segments[0];
        let rest = &segments[1..];
        
        // 处理第一级是数组路径的情况
        if first.ends_with("[]") {
            // 确保目标数组存在
            let entry = result_map
                .entry(first.to_string())
                .or_insert_with(|| serde_json::Value::Array(Vec::new()));
                
            if let serde_json::Value::Array(arr) = entry {
                // 根据值是否为数组采取不同处理策略
                match value {
                    serde_json::Value::Array(orig_elems) => {
                        // 对数组中每个元素单独处理
                        for elem in orig_elems {
                            arr.push(build_nested_object(rest, elem));
                        }
                    },
                    // 非数组值按单值处理
                    _ => arr.push(build_nested_object(rest, value)),
                }
            }
        } else {
            // 普通键或非顶层数组
            result_map.insert(first.to_string(), build_nested_object(rest, value));
        }
    }
    
    result_map
}

/// 辅助函数：从路径段和值构建嵌套对象
fn build_nested_object(segments: &[&str], value: serde_json::Value) -> serde_json::Value {
    // 从叶子开始向外包装
    segments.iter().rev().fold(value, |node, &seg| {
        let mut map = serde_json::Map::new();
        if seg.ends_with("[]") {
            // 数组路径
            map.insert(seg.to_string(), serde_json::Value::Array(vec![node]));
        } else {
            // 普通对象路径
            map.insert(seg.to_string(), node);
        }
        serde_json::Value::Object(map)
    })
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::global::rest::utils::transform::transform_salve_value;

    #[test]
    fn test_1() {
        // 示例 1
        let src1 = r#"
        {
            "Comment[]/timeline.Comment": [
                { "id": 18711, "content": "呼呼呼呼" }
            ]
        }
        "#;
        let src2 = r#"{
        "Comment[]/timeline.Comment": [
                { "id": 18711, "content": "呼呼呼呼" },
                { "id": 18712, "content": "哈哈哈哈" }
            ]
        }"#;
        let src3 = r#"{
            "Comment[]/timeline.Comment": { "id": 18711, "content": "呼呼呼呼" }
        }"#;
        let src4 = r#"{"Comment[]/User[]/timeline.User": { "id": 18711, "name": "zk" }}"#;

        // 遍历执行所有测试用例
        let test_cases = vec![src1, src2, src3, src4];
        
        for (i, src) in test_cases.iter().enumerate() {
            println!("测试用例 {}", i + 1);
            let v: serde_json::Value = serde_json::from_str(src).unwrap();
            let input_map: HashMap<String, serde_json::Value> = v.as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            
            let out = transform_salve_value(input_map);
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
            println!("----------------------------");
        }
    }
}
