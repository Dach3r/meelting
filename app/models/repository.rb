class Repository < ApplicationRecord
  SLUG_FORMAT = /\A[a-z0-9][a-z0-9._-]*\z/

  belongs_to :user
  belongs_to :license, optional: true

  enum :visibility, { private: "private", public: "public" }, default: :private, prefix: true

  normalizes :name, :description, with: ->(text_) { text_.strip }
  normalizes :slug, with: ->(slug_) { slug_.strip.downcase }

  before_validation :assign_slug, if: :will_save_change_to_name?

  validates :name, presence: true
  validates :default_branch, presence: true
  validates :description, length: { maximum: 350 }
  validate :license_is_selectable, if: :license_id_changed?
  validates :slug, presence: true, uniqueness: { scope: :user_id }, length: { maximum: 100 }, format: { with: SLUG_FORMAT }

  scope :active, -> { where(archived_at: nil) }

  def touch_pushed
    update_column(:pushed_at, Time.current)
  end

  def pushed?
    pushed_at.present?
  end

  def git_path
    Rails.configuration.x.git_root.join("#{id}.git")
  end

  def git
    Repository::Git.new(git_path)
  end

  def head_sha
    git.resolve(default_branch) if git_path.directory?
  end

  def initialized?
    head_sha.present?
  end

  private

  def assign_slug
    self.slug = name.to_s.parameterize.first(100).delete_suffix("-")
  end

  def license_is_selectable
    errors.add(:license, :inclusion) if license && !License.selectable.exists?(license.id)
  end
end
