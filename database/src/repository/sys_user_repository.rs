use anyhow::{Result, anyhow};
use sea_orm::{IntoActiveModel, QueryFilter};
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, TransactionTrait};

use crate::entity::sys_user;
use crate::entity::sys_user::ActiveModel;
use crate::get_db_unwrap;

pub async fn get_by_id(user_id: &str) -> Result<sys_user::Model> {
    let db = get_db_unwrap();
    let user = sys_user::Entity::find()
        .filter(sys_user::Column::Id.eq(user_id))
        .one(db)
        .await?;

    match user {
        Some(sys_user) => Ok(sys_user),
        None => Err(anyhow!("用户不存在")),
    }
}

pub async fn insert() -> ActiveModel {
    let db = get_db_unwrap();
    let txn = db.begin().await.unwrap();

    let user = ActiveModel {
        id: ActiveValue::NotSet,
        name: ActiveValue::Set(Some("李寻欢".to_owned())),
    }
    .save(&txn)
    .await
    .unwrap();

    txn.commit().await.unwrap();

    user
}

pub async fn delete_by_id() {
    let db = get_db_unwrap();
    let res = sys_user::Entity::delete_by_id(3).exec(db).await.unwrap();

    println!("delete by id {:?} \r\n", res.rows_affected);
}

pub async fn edit_by_id(user_id: &str) {
    let db = get_db_unwrap();

    let sys_user = get_by_id(user_id).await.unwrap();

    let mut active_model = sys_user.into_active_model();
    active_model.name = ActiveValue::Set(Some("修改后的用户名".to_owned()));
    active_model.update(db).await.unwrap();

    println!("edit success!!");
}
