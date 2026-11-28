class ProviderCredential < ApplicationRecord
  belongs_to :user

  encrypts :api_key

  enum :provider, { deepseek: "deepseek" }, default: :deepseek, validate: true
  enum :status, { unverified: "unverified", valid: "valid", invalid: "invalid" }, default: :unverified, prefix: true

  normalizes :api_key, with: ->(api_key_) { api_key_.strip }

  validates :api_key, presence: true
  validates :provider, uniqueness: { scope: :user_id }

  # Unverified keys (the provider did not answer) count as added; invalid is reserved for a future re-check.
  scope :usable, -> { where(status: %i[valid unverified]) }

  def record_verification(status, error: nil)
    self.status = status
    self.validated_at = Time.current
    self.last_error = error
  end
end
