module Organisms
  module LandingHero
    class Component < ViewComponent::Base
      private

      def initial
        ApplicationHelper::CONFIG[:name].first
      end
    end
  end
end
