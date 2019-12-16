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
    confirmed_account (id) {
        id -> Text,
        account_no -> Text,
        account_type -> Text,
        reason -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    confirmed_transaction (id) {
        id -> Text,
        reason -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

allow_tables_to_appear_in_same_query!(balance, confirmed_account, confirmed_transaction);
