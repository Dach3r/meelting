require "test_helper"

class Atoms::AgentAvatar::ComponentTest < ViewComponent::TestCase
  test "renders a decorative lime square with the bot icon" do
    avatar = render_inline(Atoms::AgentAvatar::Component.new).css("span").first

    assert_equal "true", avatar["aria-hidden"]
    assert_includes avatar["class"], "bg-lime"
    assert_includes avatar["class"], "rounded-[10px]"
    assert_equal 1, avatar.css("svg").size
  end

  test "small size shrinks the box" do
    avatar = render_inline(Atoms::AgentAvatar::Component.new(size: :sm)).css("span").first

    assert_includes avatar["class"], "size-6"
  end
end
