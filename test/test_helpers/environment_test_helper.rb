module EnvironmentTestHelper
  def in_development
    original = Rails.env
    Rails.env = "development"
    yield
  ensure
    Rails.env = original
  end
end
