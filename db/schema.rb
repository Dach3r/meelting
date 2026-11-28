# This file is auto-generated from the current state of the database. Instead
# of editing this file, please use the migrations feature of Active Record to
# incrementally modify your database, and then regenerate this schema definition.
#
# This file is the source Rails uses to define your schema when running `bin/rails
# db:schema:load`. When creating a new database, `bin/rails db:schema:load` tends to
# be faster and is potentially less error prone than running all of your
# migrations from scratch. Old migrations may fail to apply correctly if those
# migrations use external dependencies or application code.
#
# It's strongly recommended that you check this file into your version control system.

ActiveRecord::Schema[8.1].define(version: 2026_11_27_195942) do
  # These are extensions that must be enabled in order to support this database
  enable_extension "pg_catalog.plpgsql"

  create_table "access_tokens", force: :cascade do |t|
    t.bigint "user_id", null: false
    t.string "name", null: false
    t.string "token_digest", null: false
    t.string "scopes", default: [], null: false, array: true
    t.datetime "last_used_at"
    t.datetime "expires_at"
    t.datetime "revoked_at"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["token_digest"], name: "index_access_tokens_on_token_digest", unique: true
    t.index ["user_id"], name: "index_access_tokens_on_user_id"
  end

  create_table "licenses", force: :cascade do |t|
    t.string "key", null: false
    t.string "name", null: false
    t.text "description", default: "", null: false
    t.text "body", default: "", null: false
    t.string "status", default: "active", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["key"], name: "index_licenses_on_key", unique: true
    t.check_constraint "status::text = ANY (ARRAY['active'::character varying, 'inactive'::character varying]::text[])", name: "licenses_status_check"
  end

  create_table "repositories", force: :cascade do |t|
    t.string "name", null: false
    t.string "slug", null: false
    t.string "description", limit: 350
    t.string "visibility", default: "private", null: false
    t.string "default_branch", default: "main", null: false
    t.datetime "archived_at"
    t.datetime "pushed_at"
    t.bigint "license_id"
    t.bigint "user_id", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["license_id"], name: "index_repositories_on_license_id"
    t.index ["user_id", "slug"], name: "index_repositories_on_user_id_and_slug", unique: true
    t.index ["user_id"], name: "index_repositories_on_user_id"
    t.check_constraint "visibility::text = ANY (ARRAY['private'::character varying, 'public'::character varying]::text[])", name: "repositories_visibility_check"
  end

  create_table "sessions", force: :cascade do |t|
    t.string "ip_address"
    t.string "user_agent"
    t.bigint "user_id", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["user_id"], name: "index_sessions_on_user_id"
  end

  create_table "users", force: :cascade do |t|
    t.string "first_name", null: false
    t.string "last_name", null: false
    t.string "username", null: false
    t.string "email_address", null: false
    t.string "password_digest", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.datetime "onboarding_completed_at"
    t.index ["email_address"], name: "index_users_on_email_address", unique: true
    t.index ["username"], name: "index_users_on_username", unique: true
  end

  add_foreign_key "access_tokens", "users"
  add_foreign_key "repositories", "licenses"
  add_foreign_key "repositories", "users"
  add_foreign_key "sessions", "users"
end
