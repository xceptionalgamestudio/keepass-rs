use keepass::{
    db::{Entry, Group, Node, NodeRefMut, Value},
    Database, DatabaseKey,
};
use std::fs::File;
use std::path::Path;
use uuid::Uuid;

#[test]
fn test_deletion() {
    // 1. Setup
    let mut db = Database::new(Default::default());

    let mut g1 = Group::new("G1");
    let e1 = Entry::new();
    let _e1_uuid = e1.uuid;
    g1.add_child(e1);

    let mut g2 = Group::new("G2");
    let e2 = Entry::new();
    let e2_uuid = e2.uuid;
    g2.add_child(e2);
    g1.add_child(g2);

    let g1_uuid = g1.uuid;
    db.root.add_child(g1);

    let e3 = Entry::new();
    let e3_uuid = e3.uuid;
    db.root.add_child(e3);

    // 2. Test deleting a nested entry with logging
    let deleted_node = db.delete_by_uuid(&e2_uuid, true);
    assert!(deleted_node.is_some());
    if let Some(Node::Entry(e)) = deleted_node {
        assert_eq!(e.uuid, e2_uuid);
    } else {
        panic!("Expected an Entry to be deleted");
    }

    // Verify it's gone from the group
    let g1_ref = db.root.children.iter().find(|n| match n {
        Node::Group(g) => g.uuid == g1_uuid,
        _ => false,
    }).unwrap();
    if let Node::Group(g) = g1_ref {
        let g2_ref = g.children.iter().find(|n| match n {
            Node::Group(g_inner) => g_inner.name == "G2",
            _ => false,
        }).unwrap();
        if let Node::Group(g2_inner) = g2_ref {
             assert_eq!(g2_inner.children.len(), 0);
        } else {
            panic!("Expected G2 group");
        }
    } else {
        panic!("Expected G1 group");
    }


    // Verify it's in deleted_objects
    assert_eq!(db.deleted_objects.objects.len(), 1);
    assert_eq!(db.deleted_objects.objects[0].uuid, e2_uuid);

    // 3. Test deleting a group without logging
    let deleted_node = db.delete_by_uuid(&g1_uuid, false);
    assert!(deleted_node.is_some());
    if let Some(Node::Group(g)) = deleted_node {
        assert_eq!(g.uuid, g1_uuid);
        // check that it contained e1 and g2 before it was deleted
        assert_eq!(g.children.len(), 2);
    } else {
        panic!("Expected a Group to be deleted");
    }

    // Verify it's gone from the root
    assert_eq!(db.root.children.len(), 1);
    if let Some(Node::Entry(e)) = db.root.children.get(0) {
        assert_eq!(e.uuid, e3_uuid);
    } else {
        panic!("Expected E3 to be the only child of root");
    }

    // Verify deleted_objects count has not changed
    assert_eq!(db.deleted_objects.objects.len(), 1);

    // 4. Test deleting a non-existent node
    let random_uuid = Uuid::new_v4();
    let deleted_node = db.delete_by_uuid(&random_uuid, true);
    assert!(deleted_node.is_none());
    assert_eq!(db.deleted_objects.objects.len(), 1);
}


#[test]
#[cfg(feature = "save_kdbx4")]
fn test_delete_entry_and_persist() {
    let path = Path::new("test_delete_entry_and_persist.kdbx");

    // 1. Setup: Create a database with one entry in a group
    let mut db = Database::new(Default::default());
    let mut group = Group::new("Group");
    let mut entry = Entry::new();
    entry.fields.insert(
        "Title".to_string(),
        Value::Unprotected("My Entry".to_string()),
    );
    let entry_uuid = entry.uuid;
    group.add_child(entry);
    db.root.add_child(group);

    // Verify entry exists in memory before saving
    assert!(
        db.root.get(&["Group", "My Entry"]).is_some(),
        "Entry should exist in memory before first save"
    );

    // 2. Save the initial database to a temporary file
    let key = DatabaseKey::new().with_password("password");
    db.save(&mut File::create(&path).unwrap(), key.clone())
        .unwrap();

    // 3. Re-open and verify that the entry was saved
    let mut db_reopened = Database::open(&mut File::open(&path).unwrap(), key.clone()).unwrap();
    assert!(
        db_reopened.root.get(&["Group", "My Entry"]).is_some(),
        "Entry should be present after initial save and reopen"
    );

    // 4. Manually delete the entry
    if let Some(NodeRefMut::Group(group)) = db_reopened.root.get_mut(&["Group"]) {
        let original_len = group.children.len();
        group.children.retain(|node| match node {
            Node::Entry(e) => e.uuid != entry_uuid,
            _ => true,
        });
        assert_eq!(
            group.children.len(),
            original_len - 1,
            "Child entry should have been removed from in-memory db"
        );
    } else {
        panic!("Group 'Group' not found");
    }

    // 5. Save the changes back to the file
    db_reopened
        .save(&mut File::create(&path).unwrap(), key.clone())
        .unwrap();

    // 6. Re-open the database again and verify the entry is gone
    let db_final = Database::open(&mut File::open(&path).unwrap(), key.clone()).unwrap();
    assert!(
        db_final.root.get(&["Group", "My Entry"]).is_none(),
        "The entry should not exist after being deleted and saved"
    );

    // 7. Cleanup the temporary file
    std::fs::remove_file(&path).unwrap();
}

#[test]
#[cfg(feature = "save_kdbx4")]
fn test_delete_group_and_persist() {
    let path = Path::new("test_delete_group_and_persist.kdbx");

    // 1. Setup: Create a database with a group to be deleted
    let mut db = Database::new(Default::default());
    let group = Group::new("GroupToDelete");
    let group_uuid = group.uuid;
    db.root.add_child(group);

    // Verify group exists in memory before saving
    assert!(
        db.root.get(&["GroupToDelete"]).is_some(),
        "Group should exist in memory before first save"
    );

    // 2. Save the initial database to a temporary file
    let key = DatabaseKey::new().with_password("password");
    db.save(&mut File::create(&path).unwrap(), key.clone())
        .unwrap();

    // 3. Re-open and verify that the group was saved
    let mut db_reopened = Database::open(&mut File::open(&path).unwrap(), key.clone()).unwrap();
    assert!(
        db_reopened.root.get(&["GroupToDelete"]).is_some(),
        "Group should be present after initial save and reopen"
    );

    // 4. Manually delete the group
    let original_len = db_reopened.root.children.len();
    db_reopened.root.children.retain(|node| match node {
        Node::Group(g) => g.uuid != group_uuid,
        _ => true,
    });
    assert_eq!(
        db_reopened.root.children.len(),
        original_len - 1,
        "Child group should have been removed from in-memory db"
    );

    // 5. Save the changes back to the file
    db_reopened
        .save(&mut File::create(&path).unwrap(), key.clone())
        .unwrap();

    // 6. Re-open the database again and verify the group is gone
    let db_final = Database::open(&mut File::open(&path).unwrap(), key.clone()).unwrap();
    assert!(
        db_final.root.get(&["GroupToDelete"]).is_none(),
        "The group should not exist after being deleted and saved"
    );

    // 7. Cleanup the temporary file
    std::fs::remove_file(&path).unwrap();
}

// This test demonstrates how deletions are handled when merging two databases.
// It requires the `_merge` feature, which can be enabled with `cargo test --features _merge`
#[test]
#[cfg(all(feature = "save_kdbx4", feature = "_merge"))]
fn test_delete_with_merge() {
    let master_path = Path::new("test_master.kdbx");
    let replica_path = Path::new("test_replica.kdbx");

    // 1. Setup: Create a "master" database with an entry
    let mut master_db = Database::new(Default::default());
    let mut group = Group::new("Group");
    let mut entry = Entry::new();
    entry.fields.insert(
        "Title".to_string(),
        Value::Unprotected("My Entry".to_string()),
    );
    let entry_uuid = entry.uuid;
    group.add_child(entry);
    master_db.root.add_child(group);

    // 2. Save the master database
    let key = DatabaseKey::new().with_password("password");
    master_db
        .save(&mut File::create(&master_path).unwrap(), key.clone())
        .unwrap();

    // 3. Create a "replica" by opening the master db file
    let mut replica_db = Database::open(&mut File::open(&master_path).unwrap(), key.clone()).unwrap();

    // 4. In the replica, delete the entry with `log_deletion: true`
    let deleted_node = replica_db.delete_by_uuid(&entry_uuid, true);
    assert!(deleted_node.is_some());
    assert_eq!(replica_db.deleted_objects.objects.len(), 1);

    // 5. Save the replica with the logged deletion
    replica_db
        .save(&mut File::create(&replica_path).unwrap(), key.clone())
        .unwrap();

    // 6. Merge the replica's changes back into the master
    let merge_db = Database::open(&mut File::open(&replica_path).unwrap(), key.clone()).unwrap();
    master_db.merge(&merge_db).unwrap();

    // 7. Verify the entry is now deleted in the master db as well
    assert!(
        master_db.root.get(&["Group", "My Entry"]).is_none(),
        "The entry should be deleted from master after merge"
    );

    // 8. For good measure, save and re-open the master to ensure the merged change persists
    master_db
        .save(&mut File::create(&master_path).unwrap(), key.clone())
        .unwrap();
    let final_master_db = Database::open(&mut File::open(&master_path).unwrap(), key.clone()).unwrap();
    assert!(
        final_master_db.root.get(&["Group", "My Entry"]).is_none(),
        "The merged deletion should persist after saving"
    );

    // 9. Cleanup the temporary files
    std::fs::remove_file(&master_path).unwrap();
    std::fs::remove_file(&replica_path).unwrap();
}
