//! This example shows the correct way to delete entries and groups by UUID
//! and ensure that the changes are persisted to the database file.
//!
//! To run this example:
//! cargo run --example deletion --features="save_kdbx4"

use keepass::db::{Database, Entry, Group, Node, NodeRef, Value};
use keepass::DatabaseKey;
use std::fs::File;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Running Entry Deletion Example ---");
    delete_entry_by_uuid_example()?;

    println!("\n--- Running Group Deletion Example ---");
    delete_group_by_uuid_example()?;

    Ok(())
}

fn delete_entry_by_uuid_example() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("test_db_entry_delete.kdbx");
    let key = DatabaseKey::new().with_password("password");

    // 1. Create a database with a sample entry and save it.
    println!("Setting up database...");
    let mut db = Database::new(Default::default());
    let mut group = Group::new("MyGroup");
    let mut entry = Entry::new();
    entry.fields.insert("Title".to_string(), Value::Unprotected("MyEntry".to_string()));
    let entry_uuid_to_delete = entry.uuid; // Capture the UUID
    group.add_child(entry);
    db.root.add_child(group);
    db.save(&mut File::create(&path)?, key.clone())?;
    println!("Database created with entry '{}'.", entry_uuid_to_delete);

    // 2. Re-open the database.
    println!("Re-opening database to delete entry...");
    let mut db_to_modify = Database::open(&mut File::open(&path)?, key.clone())?;

    // 3. Delete the entry by its UUID.
    let deleted_node = db_to_modify.delete_by_uuid(&entry_uuid_to_delete, false);
    if deleted_node.is_none() {
        panic!("The entry should be found and deleted from memory.");
    }
    println!("Entry '{}' deleted from the database in memory.", entry_uuid_to_delete);

    // 4. IMPORTANT: Save the database to persist the deletion.
    db_to_modify.save(&mut File::create(&path)?, key.clone())?;
    println!("Changes saved to disk.");

    // 5. Re-open the database again to verify.
    println!("Re-opening database for verification...");
    let final_db = Database::open(&mut File::open(&path)?, key.clone())?;

    // Check that the entry is gone by iterating through all nodes.
    let found_entry = final_db.root.iter().any(|node| match node {
        NodeRef::Entry(e) => e.uuid == entry_uuid_to_delete,
        _ => false,
    });

    if !found_entry {
        println!("SUCCESS: Entry '{}' is confirmed to be deleted from the file.", entry_uuid_to_delete);
    } else {
        eprintln!("FAILURE: Entry '{}' was found in the database file.", entry_uuid_to_delete);
    }

    // Cleanup
    std::fs::remove_file(&path)?;

    Ok(())
}

fn delete_group_by_uuid_example() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("test_db_group_delete.kdbx");
    let key = DatabaseKey::new().with_password("password");

    // 1. Create a database with a sample group and save it.
    println!("Setting up database...");
    let mut db = Database::new(Default::default());
    let group = Group::new("GroupToDelete");
    let group_uuid_to_delete = group.uuid; // Capture the UUID
    db.root.add_child(group);
    db.save(&mut File::create(&path)?, key.clone())?;
    println!("Database created with group '{}'.", group_uuid_to_delete);

    // 2. Re-open the database.
    println!("Re-opening database to delete group...");
    let mut db_to_modify = Database::open(&mut File::open(&path)?, key.clone())?;

    // 3. Delete the group by its UUID.
    let deleted_node = db_to_modify.delete_by_uuid(&group_uuid_to_delete, false);
    if deleted_node.is_none() {
        panic!("The group should be found and deleted from memory.");
    }
    println!("Group '{}' deleted from the database in memory.", group_uuid_to_delete);

    // 4. IMPORTANT: Save the database to persist the deletion.
    db_to_modify.save(&mut File::create(&path)?, key.clone())?;
    println!("Changes saved to disk.");

    // 5. Re-open the database again to verify.
    println!("Re-opening database for verification...");
    let final_db = Database::open(&mut File::open(&path)?, key.clone())?;

    // Check that the group is gone.
    let found_group = final_db.root.iter().any(|node| match node {
        NodeRef::Group(g) => g.uuid == group_uuid_to_delete,
        _ => false,
    });

    if !found_group {
        println!("SUCCESS: Group '{}' is confirmed to be deleted from the file.", group_uuid_to_delete);
    } else {
        eprintln!("FAILURE: Group '{}' was found in the database file.", group_uuid_to_delete);
    }

    // Cleanup
    std::fs::remove_file(&path)?;

    Ok(())
}
