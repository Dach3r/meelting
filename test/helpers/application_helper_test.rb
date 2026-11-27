require "test_helper"

class ApplicationHelperTest < ActionView::TestCase
  test "format_cents renders dollars from integer cents" do
    assert_equal "$1.84", format_cents(184)
    assert_equal "$0.05", format_cents(5)
    assert_equal "$12.00", format_cents(1200)
  end

  test "format_duration renders minutes and padded seconds" do
    assert_equal "11m 42s", format_duration(702)
    assert_equal "0m 05s", format_duration(5)
  end

  test "format_clock renders mm:ss" do
    assert_equal "07:58", format_clock(478)
  end
end
