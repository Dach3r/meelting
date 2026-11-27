require "test_helper"

class ApplicationHelperTest < ActionView::TestCase
  test "format_duration renders minutes and padded seconds" do
    assert_equal "11m 42s", format_duration(702)
    assert_equal "0m 05s", format_duration(5)
  end
end
