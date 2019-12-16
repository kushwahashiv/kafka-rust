table! {
    balance (id) {
        id -> Text,
        account_no -> Text,
        amount -> Double,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    account (id) {
        id -> Text,
        account_no -> Text,
        account_type -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

allow_tables_to_appear_in_same_query!(balance, account);
