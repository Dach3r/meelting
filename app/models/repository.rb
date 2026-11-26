class Repository < ApplicationRecord
  SLUG_FORMAT = /\A[a-z0-9][a-z0-9._-]*\z/

  belongs_to :user
  belongs_to :license, optional: true

  enum :visibility, { private: "private", public: "public" }, default: :private, prefix: true

  normalizes :name, :description, with: ->(v) { v.strip }
  normalizes :slug, with: ->(s) { s.strip.downcase }

  validates :name, presence: true
  validates :default_branch, presence: true
  validates :description, length: { maximum: 350 }
  validate :license_is_active, if: :license_id_changed?
  validates :slug, presence: true, uniqueness: { scope: :user_id }, length: { maximum: 100 }, format: { with: SLUG_FORMAT }

  scope :active, -> { where(archived_at: nil) }
  scope :archived, -> { where.not(archived_at: nil) }

  def archived?
    archived_at.present?
  end

  def archive!
    update!(archived_at: Time.current) unless archived?
  end

  def unarchive!
    update!(archived_at: nil) if archived?
  end

  def touch_pushed
    update_column(:pushed_at, Time.current)
  end

  private

  def license_is_active
    errors.add(:license, :inclusion) if license&.inactive?
  end
end
