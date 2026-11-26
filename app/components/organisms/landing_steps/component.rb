module Organisms
  module LandingSteps
    class Component < ViewComponent::Base
      STEPS = %w[assign sandbox approve merge].freeze
    end
  end
end
