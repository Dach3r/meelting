class CreateProviderCredentials < ActiveRecord::Migration[8.1]
  def change
    create_table :provider_credentials do |t|
      t.references :user, null: false, foreign_key: true, index: false
      t.string :provider, null: false
      t.text :api_key, null: false
      t.string :status, null: false, default: "unverified"
      t.datetime :validated_at
      t.string :last_error

      t.timestamps

      t.check_constraint "provider IN ('deepseek')", name: "provider_credentials_provider_check"
      t.check_constraint "status IN ('unverified', 'valid', 'invalid')", name: "provider_credentials_status_check"
    end

    add_index :provider_credentials, %i[user_id provider], unique: true
  end
end
