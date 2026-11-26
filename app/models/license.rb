class License < ApplicationRecord
  has_many :repositories, dependent: :restrict_with_error

  enum :status, { active: "active", inactive: "inactive" }, default: :active

  normalizes :key, with: ->(k) { k.strip.downcase }
  normalizes :name, with: ->(n) { n.strip }

  validates :key, presence: true, uniqueness: true, format: { with: /\A[a-z0-9][a-z0-9.-]*\z/ }
  validates :name, presence: true
end
