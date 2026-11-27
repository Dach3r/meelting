class License < ApplicationRecord
  has_many :repositories, dependent: :restrict_with_error

  enum :status, { active: "active", inactive: "inactive" }, default: :active

  normalizes :key, with: ->(key_) { key_.strip.downcase }
  normalizes :name, with: ->(name_) { name_.strip }

  validates :key, presence: true, uniqueness: true, format: { with: /\A[a-z0-9][a-z0-9.-]*\z/ }
  validates :name, presence: true
end
