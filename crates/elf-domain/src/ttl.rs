pub fn compute_expires_at(
    ttl_days: Option<i64>,
    note_type: &str,
    cfg: &elf_config::Config,
    now: time::OffsetDateTime,
) -> Option<time::OffsetDateTime> {
    let days = if let Some(value) = ttl_days.filter(|days| *days > 0) {
        value
    } else {
        match note_type {
            "plan" => cfg.lifecycle.ttl_days.plan,
            "fact" => cfg.lifecycle.ttl_days.fact,
            "preference" => cfg.lifecycle.ttl_days.preference,
            "constraint" => cfg.lifecycle.ttl_days.constraint,
            "decision" => cfg.lifecycle.ttl_days.decision,
            "profile" => cfg.lifecycle.ttl_days.profile,
            _ => 0,
        }
    };

    if days > 0 {
        Some(now + time::Duration::days(days))
    } else {
        None
    }
}
