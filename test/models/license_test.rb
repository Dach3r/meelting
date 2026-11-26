require "test_helper"

class LicenseTest < ActiveSupport::TestCase
  test "valid license" do
    assert License.new(key: "apache-2.0", name: "Apache License 2.0").valid?
  end

  test "requires key and name" do
    license = License.new
    assert_not license.valid?
    assert license.errors[:key].any?
    assert license.errors[:name].any?
  end

  test "normalizes key" do
    assert_equal "apache-2.0", License.new(key: " Apache-2.0 ").key
  end

  test "rejects duplicate key regardless of case" do
    license = License.new(key: "MIT", name: "Other")
    assert_not license.valid?
    assert license.errors[:key].any?
  end

  test "status defaults to active" do
    assert License.new.active?
  end

  test "rejects unknown status" do
    assert_raises(ArgumentError) { License.new(status: "bogus") }
  end

  test "cannot be destroyed while repositories use it" do
    assert_no_difference "License.count" do
      assert_not licenses(:agpl).destroy
    end
  end
end
