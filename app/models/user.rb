class User < ApplicationRecord
  has_secure_password

  has_many :sessions, dependent: :destroy
  has_many :access_tokens, dependent: :destroy
  has_many :repositories, dependent: :destroy
  has_many :provider_credentials, dependent: :destroy

  normalizes :username, with: ->(username_) { username_.strip }
  normalizes :email_address, with: ->(email_address_) { email_address_.strip.downcase }
  normalizes :first_name, :last_name, with: ->(name_) { name_.strip }

  validates :first_name, :last_name, presence: true
  validates :password, length: { minimum: 8 }, allow_nil: true
  validates :email_address, presence: true, uniqueness: true, format: { with: URI::MailTo::EMAIL_REGEXP }
  validates :username, presence: true, uniqueness: true, length: { in: 3..39 }, format: { with: /\A[a-z][a-z0-9_]*\z/ }

  def self.username_errors(username)
    new(username: username).tap(&:valid?).errors[:username]
  end

  def full_name
    "#{first_name} #{last_name}"
  end

  def initials
    "#{first_name[0]}#{last_name[0]}".upcase
  end
end
