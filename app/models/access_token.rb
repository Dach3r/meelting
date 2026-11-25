class AccessToken < ApplicationRecord
  PREFIX = "mlt_".freeze
  SCOPES = %w[git_read git_write api].freeze
  LAST_USED_THROTTLE = 1.hour

  belongs_to :user

  attr_reader :plain_token

  before_validation :generate_token, on: :create

  validates :name, presence: true
  validates :token_digest, presence: true, uniqueness: true
  validate :scopes_are_known

  scope :active, -> { where(revoked_at: nil).where("expires_at IS NULL OR expires_at > ?", Time.current) }

  def self.digest(plain_token)
    Digest::SHA256.hexdigest(plain_token.to_s)
  end

  def self.authenticate(plain_token, scope:)
    return if plain_token.blank?

    token = active.find_by(token_digest: digest(plain_token))
    return unless token&.scopes&.include?(scope.to_s)

    token.touch_last_used
    token
  end

  def revoke!
    update!(revoked_at: Time.current) unless revoked?
  end

  def revoked?
    revoked_at.present?
  end

  def expired?
    expires_at.present? && expires_at <= Time.current
  end

  def active?
    !revoked? && !expired?
  end

  def touch_last_used
    return if last_used_at && last_used_at > LAST_USED_THROTTLE.ago

    update_column(:last_used_at, Time.current)
  end

  private
  def generate_token
    return if token_digest.present?

    @plain_token = "#{PREFIX}#{SecureRandom.urlsafe_base64(32)}"
    self.token_digest = self.class.digest(@plain_token)
  end

  def scopes_are_known
    self.scopes = Array(scopes).compact_blank.uniq
    errors.add(:scopes, :blank) if scopes.empty?
    errors.add(:scopes, :inclusion) unless (scopes - SCOPES).empty?
  end
end
