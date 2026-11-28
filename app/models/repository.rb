class Repository < ApplicationRecord
  SLUG_FORMAT = /\A[a-z0-9][a-z0-9._-]*\z/

  belongs_to :user
  belongs_to :license, optional: true

  enum :visibility, { private: "private", public: "public" }, default: :private, prefix: true

  normalizes :name, :description, with: ->(text_) { text_.strip }
  normalizes :slug, with: ->(slug_) { slug_.strip.downcase }

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

  def pushed?
    pushed_at.present?
  end

  def git_path
    Rails.configuration.x.git_root.join("#{id}.git")
  end

  def file_on_default_branch?(path)
    return false unless git_path.directory? && valid_branch_name?(default_branch)

    _output, status = Open3.capture2e("git", "--git-dir", git_path.to_s, "cat-file", "-e", "refs/heads/#{default_branch}:#{path}")
    status.success?
  end

  private
  def valid_branch_name?(name)
    return false if name.start_with?("-")

    _output, status = Open3.capture2e("git", "check-ref-format", "--branch", name)
    status.success?
  end

  def license_is_active
    errors.add(:license, :inclusion) if license&.inactive?
  end
end
