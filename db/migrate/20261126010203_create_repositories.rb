class CreateRepositories < ActiveRecord::Migration[8.1]
  def change
    create_table :repositories do |t|
      t.string :name, null: false
      t.string :slug, null: false
      t.string :description, limit: 350
      t.string :visibility, null: false, default: "private"
      t.string :default_branch, null: false, default: "main"
      t.datetime :archived_at
      t.datetime :pushed_at
      t.references :license, foreign_key: true
      t.references :user, null: false, foreign_key: true

      t.timestamps

      t.check_constraint "visibility IN ('private', 'public')", name: "repositories_visibility_check"
    end

    add_index :repositories, %i[user_id slug], unique: true
  end
end
