table! {
      transactions (id) {
        id -> Text,
        account_no -> Text,
        amount -> Double,
        new_balance -> Double,
        account_type -> Text,
        changed_by -> Text,
        from_to -> Text,
        description -> Text,
        created_at -> Timestamp,
    }
}

table! {
  account (id) {
      id -> Text,
      username -> Text,
      password -> Text,
      created_at -> Timestamp,
  }
}

allow_tables_to_appear_in_same_query!(transactions, account);
