class User < ApplicationRecord
  has_secure_password

  has_many :sessions, dependent: :destroy
  has_many :access_tokens, dependent: :destroy
  has_many :repositories, dependent: :destroy

  normalizes :username, with: ->(u) { u.strip }
  normalizes :email_address, with: ->(e) { e.strip.downcase }
  normalizes :first_name, :last_name, with: ->(n) { n.strip }

  validates :first_name, :last_name, presence: true
  validates :username, presence: true,
                       uniqueness: true,
                       length: { in: 3..39 },
                       format: { with: /\A[a-z_]+\z/ }
  validates :password, length: { minimum: 8 }, allow_nil: true
  validates :email_address, presence: true,
                            uniqueness: true,
                            format: { with: URI::MailTo::EMAIL_REGEXP }

  def self.username_errors(username)
    new(username: username).tap(&:valid?).errors[:username]
  end

  def full_name
    "#{first_name} #{last_name}"
  end
end
