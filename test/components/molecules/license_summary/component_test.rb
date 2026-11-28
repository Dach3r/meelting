require "test_helper"

class Molecules::LicenseSummary::ComponentTest < ViewComponent::TestCase
  test "renders name, description and the three rule columns" do
    license = Repositories::DetectLicense::Result.new(key: "agpl-3.0", path: "LICENSE")
    node = render_inline(Molecules::LicenseSummary::Component.new(license: license))

    assert_equal "GNU Affero General Public License v3.0", node.css("h2").text.strip
    assert_equal license.description, node.css("h2 + p").text
    assert_equal %w[permissions conditions limitations], node.css("[data-group]").map { |group_| group_["data-group"] }
    assert_includes node.css("[data-group=conditions] li").map(&:text), "El uso en red es distribución"
  end
end
