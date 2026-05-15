use rusqlite::Connection;
use tempfile::NamedTempFile;
use vire_lib::{
    archive_project_repo, create_entry_repo, create_project_repo, export_csv_repo, init_db,
    list_entries_repo, summary_repo, update_entry_repo, ProjectInput, TimeEntryInput,
};

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    init_db(&c).unwrap();
    c
}

fn entry_input(project_id: String, note: Option<String>) -> TimeEntryInput {
    TimeEntryInput {
        project_id,
        date: "2026-03-14".into(),
        start_time: "09:00".into(),
        end_time: "10:00".into(),
        note,
    }
}

#[test]
fn archived_project_entries_remain_editable_for_historical_corrections() {
    let c = conn();
    let project = create_project_repo(
        &c,
        ProjectInput {
            name: "Historical Client".into(),
            notes: None,
        },
    )
    .unwrap();
    let entry = create_entry_repo(&c, entry_input(project.id.clone(), Some("before archive".into()))).unwrap();

    archive_project_repo(&c, project.id.clone()).unwrap();

    let updated = update_entry_repo(
        &c,
        entry.id,
        TimeEntryInput {
            project_id: project.id,
            date: "2026-03-14".into(),
            start_time: "09:00".into(),
            end_time: "10:30".into(),
            note: Some("corrected after archive".into()),
        },
    );

    assert!(
        updated.is_ok(),
        "historical entries for archived projects should remain editable; got {updated:?}"
    );
    let updated = updated.unwrap();
    assert_eq!(updated.duration_minutes, 90);
    assert_eq!(updated.note.as_deref(), Some("corrected after archive"));
}

#[test]
fn report_operations_reject_inverted_date_ranges_instead_of_silently_returning_empty_data() {
    let c = conn();
    let project = create_project_repo(
        &c,
        ProjectInput {
            name: "Range Client".into(),
            notes: None,
        },
    )
    .unwrap();
    create_entry_repo(&c, entry_input(project.id, None)).unwrap();

    assert!(
        list_entries_repo(&c, "2026-03-15".into(), "2026-03-14".into(), None).is_err(),
        "listing entries with start date after end date should be a validation error"
    );
    assert!(
        summary_repo(&c, "2026-03-15".into(), "2026-03-14".into(), None).is_err(),
        "summaries with start date after end date should be a validation error"
    );

    let out = NamedTempFile::new().unwrap();
    assert!(
        export_csv_repo(
            &c,
            "2026-03-15".into(),
            "2026-03-14".into(),
            None,
            out.path()
        )
        .is_err(),
        "CSV export with start date after end date should be a validation error"
    );
}
