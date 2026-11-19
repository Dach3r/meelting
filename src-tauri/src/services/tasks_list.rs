use crate::domain::file_copy::FileCopy;
use crate::domain::task::{Task, TaskRoomLink};
use crate::domain::worktree::Worktree;
use crate::repositories::{
    task_agents, task_room_links, task_worktree_file_copies, task_worktrees, tasks,
};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<Vec<Task>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin().await?;
    let mut list = tasks::list(&mut *transaction, &workspace_id).await?;
    let assignments = task_agents::list(&mut *transaction, &workspace_id).await?;
    let links = task_room_links::list(&mut *transaction, &workspace_id).await?;
    let mut worktrees =
        task_worktrees::active_in_workspace(&mut *transaction, &workspace_id).await?;
    transaction.commit().await?;
    for task in &mut list {
        with_agents(task, &assignments);
        with_links(task, &links);
        if let Some(index) = worktrees
            .iter()
            .position(|(task_id, _)| *task_id == task.id)
        {
            task.worktree = Some(worktrees.swap_remove(index).1);
        }
    }
    Ok(list)
}

pub async fn one(pool: &SqlitePool, workspace_id: &str, id: &str) -> Result<Task, SessionError> {
    let mut transaction = pool.begin().await?;
    let mut task = tasks::get(&mut *transaction, workspace_id, id)
        .await?
        .ok_or(SessionError::TaskNotFound)?;
    let assignments = task_agents::list(&mut *transaction, workspace_id).await?;
    let links = task_room_links::list(&mut *transaction, workspace_id).await?;
    task.worktree = task_worktrees::active_for_task(&mut *transaction, workspace_id, id).await?;
    let copies =
        task_worktree_file_copies::active_in_workspace(&mut *transaction, workspace_id).await?;
    transaction.commit().await?;
    if let Some(worktree) = task.worktree.as_mut() {
        with_copies(worktree, &copies);
    }
    with_agents(&mut task, &assignments);
    with_links(&mut task, &links);
    Ok(task)
}

fn with_copies(worktree: &mut Worktree, copies: &[(String, FileCopy)]) {
    worktree.copies = copies
        .iter()
        .filter(|(worktree_id, _)| *worktree_id == worktree.id)
        .map(|(_, copy)| copy.clone())
        .collect();
}

fn with_links(task: &mut Task, links: &[(String, TaskRoomLink)]) {
    task.linked_rooms = links
        .iter()
        .filter(|(task_id, _)| *task_id == task.id)
        .map(|(_, link)| link.clone())
        .collect();
}

fn with_agents(task: &mut Task, assignments: &[(String, String)]) {
    task.agent_ids = assignments
        .iter()
        .filter(|(task_id, _)| *task_id == task.id)
        .map(|(_, agent_id)| agent_id.clone())
        .collect();
}
