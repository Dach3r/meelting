class CreateLicenses < ActiveRecord::Migration[8.1]
  def change
    create_table :licenses do |t|
      t.string :key, null: false
      t.string :name, null: false
      t.text :description, null: false, default: ""
      t.string :status, null: false, default: "active"

      t.timestamps

      t.check_constraint "status IN ('active', 'inactive')", name: "licenses_status_check"
    end

    add_index :licenses, :key, unique: true
  end
end
