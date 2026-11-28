ENV["RAILS_ENV"] ||= "test"
require_relative "../config/environment"
require "rails/test_help"
require_relative "test_helpers/session_test_helper"
require_relative "test_helpers/environment_test_helper"
require_relative "test_helpers/git_test_helper"
require_relative "test_helpers/provider_test_helper"

module ActiveSupport
  class TestCase
    include SessionTestHelper
    include EnvironmentTestHelper
    include GitTestHelper
    include ProviderTestHelper

    parallelize(workers: :number_of_processors)

    fixtures :all
  end
end
