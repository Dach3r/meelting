module Molecules
  module PhaseNotice
    class Component < ViewComponent::Base
      def initialize(title:, body:, phase:)
        @title = title
        @body = body
        @phase = phase
      end

      private
      attr_reader :title, :body, :phase
    end
  end
end
