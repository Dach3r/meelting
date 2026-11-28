module Molecules
  module TokenScopes
    class Component < ViewComponent::Base
      def initialize(access_token:)
        @access_token = access_token
      end

      private
      attr_reader :access_token

      def field_id(scope)
        "push_token_scope_#{scope}"
      end
    end
  end
end
