#[test]
fn can_add_to_cgroup() {
    use std::process;
    use super::cgroup_add_pid;

    let pid = process::id();
    cgroup_add_pid("league_client", pid).unwrap();
}
