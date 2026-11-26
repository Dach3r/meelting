require "test_helper"

class UserTest < ActiveSupport::TestCase
  test "valid user" do
    user = User.new(username: "grace", first_name: "Grace", last_name: "Hopper", email_address: "new@example.com", password: "password123", password_confirmation: "password123")
    assert user.valid?
  end

  test "normalizes email address" do
    user = User.new(email_address: "  NEW@Example.COM ")
    assert_equal "new@example.com", user.email_address
  end

  test "requires first and last name" do
    user = User.new(email_address: "new@example.com", password: "password123")
    assert_not user.valid?
    assert user.errors[:first_name].any?
    assert user.errors[:last_name].any?
  end

  test "strips names" do
    user = User.new(first_name: "  Grace ", last_name: " Hopper  ")
    assert_equal "Grace", user.first_name
    assert_equal "Hopper", user.last_name
  end

  test "requires email address" do
    user = User.new(password: "password123")
    assert_not user.valid?
    assert user.errors[:email_address].any?
  end

  test "rejects invalid email format" do
    user = User.new(email_address: "not-an-email", password: "password123")
    assert_not user.valid?
    assert user.errors[:email_address].any?
  end

  test "rejects duplicate email regardless of case" do
    user = User.new(email_address: users(:one).email_address.upcase, password: "password123")
    assert_not user.valid?
    assert user.errors[:email_address].any?
  end

  test "rejects short password" do
    user = User.new(email_address: "new@example.com", password: "short")
    assert_not user.valid?
    assert user.errors[:password].any?
  end

  test "authenticate_by with valid credentials" do
    assert_equal users(:one), User.authenticate_by(username: "ada", password: "password123")
  end

  test "authenticate_by with wrong password" do
    assert_nil User.authenticate_by(username: "ada", password: "wrong")
  end

  test "normalizes username" do
    assert_equal "grace", User.new(username: "  Grace ").username
  end

  test "requires username" do
    user = User.new(password: "password123")
    assert_not user.valid?
    assert user.errors[:username].any?
  end

  test "rejects duplicate username regardless of case" do
    user = User.new(username: users(:one).username.upcase, password: "password123")
    assert_not user.valid?
    assert user.errors[:username].any?
  end

  test "rejects invalid username format or length" do
    %w[ab -grace gra.ce grace!].each do |username|
      user = User.new(username: username, password: "password123")
      assert_not user.valid?, "#{username} should be invalid"
      assert user.errors[:username].any?
    end
  end

  test "destroying user destroys repositories" do
    user = users(:one)
    assert_difference "Repository.count", -user.repositories.count do
      user.destroy
    end
  end

  test "destroying user destroys sessions" do
    user = users(:one)
    user.sessions.create!
    assert_difference "Session.count", -1 do
      user.destroy
    end
  end

  test "full_name joins first and last name" do
    assert_equal "Ada Lovelace", users(:one).full_name
  end
end
