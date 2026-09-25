Feature: Harbor methodology conformance
  As the Shipshape workflow itself
  I want executable checks of my own methodology rules
  So that violations become failing verification targets instead of silent drift

  @captain @conformance
  Scenario: The watchbill respects its fixed shape
    Given the watchbill file at "watchbill.json" when present
    When the verifier reads every watch object
    Then every key matches "watch<number>" with only a "scenarios" array
    And every reference follows the "<spec>.feature:<Scenario Name>" form
    And an absent watchbill conforms as the deck at rest

  @captain @conformance
  Scenario: The implementation carries no standing perturbation token
    Given the implementation directory "src"
    When the verifier searches every source file for the token "PERTURBATION"
    Then no match is found
