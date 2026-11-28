module Molecules
  module LicenseSummary
    class Component < ViewComponent::Base
      def initialize(license:)
        @license = license
      end

      private
      attr_reader :license
    end
  end
end
