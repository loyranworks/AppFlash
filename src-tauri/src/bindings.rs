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

/// Startup and resume keep usable shortcuts even if another application owns one.
/// Saving a changed configuration still uses the atomic `transition` above.
pub struct Enabled {
    pub registered: Vec<String>,
    failures: Vec<String>,
}

impl Enabled {
    pub fn paused(&self) -> bool {
        self.registered.is_empty() && !self.failures.is_empty()
    }

    pub fn warning(&self) -> Option<String> {
        if self.failures.is_empty() {
            return None;
        }
        let summary = if self.paused() {
            "所有已配置的快捷键均注册失败，快捷切换已暂停。".to_owned()
        } else {
            format!(
                "部分快捷键未能注册；{} 个可用快捷键仍可切换。",
                self.registered.len()
            )
        };
        Some(format!(
            "{summary}请关闭占用程序或修改 Prefix / 字母后保存。{}",
            self.failures.join("；")
        ))
    }
}

pub fn enable_available(
    current: &[String],
    desired: &[String],
    registry: &impl Registry,
) -> Enabled {
    let mut outcome = Enabled {
        registered: current.to_vec(),
        failures: Vec::new(),
    };
    for key in desired {
        if outcome.registered.contains(key) {
            continue;
        }
        match registry.register(key) {
            Ok(()) => outcome.registered.push(key.clone()),
            Err(error) => outcome
                .failures
                .push(format!("快捷键 {key} 注册失败：{error}")),
        }
    }
    outcome
}
