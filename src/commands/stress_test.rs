use crate::{Context, Error};
use std::sync::Arc;

#[poise::command(slash_command, owners_only)]
pub async fn stress_test(ctx: Context<'_>) -> Result<(), Error> {
    const TASKS: usize = 1000;

    let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut handles = Vec::with_capacity(TASKS);

    for _ in 0..TASKS {
        let counter = Arc::clone(&counter);

        handles.push(tokio::spawn(async move {
            counter.fetch_add(
                1,
                std::sync::atomic::Ordering::Relaxed,
            );
        }));
    }

    for handle in handles {
        handle.await?;
    }

    let result = counter.load(std::sync::atomic::Ordering::Relaxed);

    ctx.say(format!(
        "Stress test: {TASKS} tareas\nResultado: {result}\n{}",
        if result == TASKS as u64 {
            "✅ TEST APROBADO"
        } else {
            "❌ TEST FALLIDO"
        }
    ))
    .await?;

    Ok(())
}