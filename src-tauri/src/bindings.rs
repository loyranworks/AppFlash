pub trait Registry {
    fn register(&self, key: &str) -> Result<(), String>;
    fn unregister(&self, key: &str) -> Result<(), String>;
}

pub fn transition(
    old: &[String],
    new: &[String],
    registry: &impl Registry,
    persist: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let mut added: Vec<String> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    let result = (|| {
        for key in new.iter().filter(|key| !old.contains(key)) {
            registry
                .register(key)
                .map_err(|error| format!("快捷键 {key} 注册失败：{error}"))?;
            added.push(key.clone());
        }
        for key in old.iter().filter(|key| !new.contains(key)) {
            registry
                .unregister(key)
                .map_err(|error| format!("快捷键 {key} 释放失败：{error}"))?;
            removed.push(key.clone());
        }
        persist()
    })();
    if let Err(error) = result {
        let mut failures = Vec::new();
        for key in &added {
            if let Err(failure) = registry.unregister(key) {
                failures.push(failure);
            }
        }
        for key in &removed {
            if let Err(failure) = registry.register(key) {
                failures.push(failure);
            }
        }
        return if failures.is_empty() {
            Err(format!("{error}；原绑定保持不变"))
        } else {
            Err(format!(
                "{error}；回滚失败，请退出后重新打开：{}",
                failures.join("；")
            ))
        };
    }
    Ok(())
}
