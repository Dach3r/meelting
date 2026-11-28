class License < ApplicationRecord
  SELECTABLE_KEYS = %w[
    apache-2.0 gpl-3.0 mit bsd-2-clause bsd-3-clause bsl-1.0 cc0-1.0
    epl-2.0 agpl-3.0 gpl-2.0 lgpl-2.1 mpl-2.0 unlicense
  ].freeze
  PLACEHOLDERS = %w[[year] [fullname]].freeze
  TEMPLATES_PATH = Rails.root.join("vendor/licenses")

  has_many :repositories, dependent: :restrict_with_error

  enum :status, { active: "active", inactive: "inactive" }, default: :active

  normalizes :key, with: ->(key_) { key_.strip.downcase }
  normalizes :name, with: ->(name_) { name_.strip }

  validates :key, presence: true, uniqueness: true, format: { with: /\A[a-z0-9][a-z0-9.-]*\z/ }
  validates :name, presence: true

  scope :selectable, -> { active.where(key: SELECTABLE_KEYS).in_order_of(:key, SELECTABLE_KEYS) }

  def self.sync_catalog
    SELECTABLE_KEYS.each do |key_|
      metadata = Licensee::License.find(key_)
      find_or_initialize_by(key: key_).update!(name: metadata.name, description: metadata.meta.description)
    end
  end

  def template
    TEMPLATES_PATH.join("#{key}.txt").read
  end

  def render(user:, year: Time.current.year)
    values = { "[year]" => year.to_s, "[fullname]" => user.full_name.presence || user.username }
    text = template.gsub("\r\n", "\n").gsub(Regexp.union(PLACEHOLDERS), values)
    text.end_with?("\n") ? text : "#{text}\n"
  end
end
