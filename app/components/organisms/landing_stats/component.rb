module Organisms
  module LandingStats
    class Component < ViewComponent::Base
      STATS = %w[sandbox secrets providers license].freeze
    end
  end
end
