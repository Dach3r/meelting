require "test_helper"

class LicenseTest < ActiveSupport::TestCase
  # Placeholders that explain how to apply the license to source files (Apache-2.0 appendix);
  # they are not filled when the LICENSE file is created.
  KEPT_TOKENS = %w[[yyyy]].freeze

  def template_license(key)
    License.new(key: key, name: key)
  end

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

  test "every selectable key has a template" do
    License::SELECTABLE_KEYS.each do |key_|
      assert License::TEMPLATES_PATH.join("#{key_}.txt").file?, "missing template for #{key_}"
    end
  end

  test "every selectable template only uses placeholders that are handled" do
    License::SELECTABLE_KEYS.each do |key_|
      tokens = template_license(key_).template.scan(/\[[a-z]+\]/).uniq
      unhandled = tokens - License::PLACEHOLDERS - KEPT_TOKENS

      assert_empty unhandled, "#{key_} has unhandled placeholders"
    end
  end

  test "selectable keeps the configured order and skips inactive licenses" do
    License.sync_catalog
    licenses(:mit).update!(status: :inactive)

    assert_equal License::SELECTABLE_KEYS - ["mit"], License.selectable.map(&:key)
  end

  test "sync_catalog takes names and descriptions from the license metadata" do
    License.sync_catalog

    assert_equal "Apache License 2.0", License.find_by!(key: "apache-2.0").name
    assert_equal Licensee::License.find("mit").meta.description, License.find_by!(key: "mit").description
  end

  test "render fills the year and the full name" do
    text = licenses(:mit).render(user: users(:one), year: 2026)

    assert_includes text, "Copyright (c) 2026 Ada Lovelace"
    assert_no_match(/\[year\]|\[fullname\]/, text)
    assert text.end_with?("\n")
    assert_not_includes text, "\r"
  end

  test "render falls back to the username without a full name" do
    user = User.new(username: "ada", first_name: "", last_name: "")

    assert_includes template_license("bsd-3-clause").render(user: user, year: 2026), "Copyright (c) 2026, ada"
  end

  test "render leaves the apache appendix untouched" do
    license = template_license("apache-2.0")

    assert_equal license.template, license.render(user: users(:one), year: 2026)
    assert_includes license.template, "Copyright [yyyy] [name of copyright owner]"
  end

  test "render leaves the gpl how-to-apply placeholders untouched" do
    assert_includes template_license("gpl-3.0").render(user: users(:one), year: 2026), "Copyright (C) <year>  <name of author>"
  end
end
