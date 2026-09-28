Feature: Finger init
  As the single user installing thimbl
  I want a command that establishes the card state directory and keeps my classic dot-files working
  So that a caged or hookless deployment starts with my real card and my editor keeps working

  Scenario: Init seeds the state directory from the home dot-files and links them
    Given the home has a project file with content "Deploying thimbl"
    And the home has a plan file with content "Ship the finger server"
    When thimbl init runs
    Then the state directory has the project content "Deploying thimbl"
    And the state directory has the plan content "Ship the finger server"
    And the home project links to the state file
    And the home plan links to the state file

  Scenario: Init keeps an existing state file
    Given the state directory has a plan file with content "Already published"
    When thimbl init runs
    Then the state directory has the plan content "Already published"

  Scenario: Init reports a conflicting home plan and leaves it alone
    Given the state directory has a plan file with content "Published"
    And the home has a plan file with content "Unpublished draft"
    When thimbl init runs
    Then the home plan is still a regular file
    And the init output names a conflict

  Scenario: Init leaves a home symlink pointing elsewhere alone
    Given the home plan is a symlink to elsewhere
    When thimbl init runs
    Then the home plan is still a symlink to elsewhere
    And the init output names the home plan

  Scenario: Init force relinks a conflicting home plan and keeps its content
    Given the state directory has a plan file with content "Published"
    And the home has a plan file with content "Unpublished draft"
    When thimbl init runs with "--force"
    Then the state directory has the plan content "Unpublished draft"
    And the home plan links to the state file

  Scenario: A second init run reports kept and changes nothing
    Given the home has a plan file with content "Ship the finger server"
    When thimbl init runs
    And thimbl init runs again
    Then the init output names the plan kept
    And the init run exits with code 0
    And the state directory has the plan content "Ship the finger server"
    And the home plan links to the state file

  Scenario: Writing through the home link after init is served live
    Given the home has a plan file with content "Ship the finger server"
    And thimbl init has run
    And the finger server is running on an ephemeral port
    And the plan file content is changed to "Updated plan"
    When a client connects and sends the user's login name
    Then the response contains a "Plan:" section with "Updated plan"

  Scenario: Init with no home dot-files creates empty state files and links them
    Given the state directory has no plan file
    And the home has no plan file
    When thimbl init runs
    Then the state directory has an empty plan file
    And the home plan links to the state file

  Scenario: An empty state file serves a blank plan section
    Given the state directory has a plan file with content ""
    And the finger server is running on an ephemeral port
    When a client connects and sends the user's login name
    Then the response contains "Plan:"
    And the response does not contain "No Plan."

  Scenario: Init with no-link touches only the state directory
    Given the home has a plan file with content "Ship the finger server"
    When thimbl init runs with "--no-link"
    Then the state directory has the plan content "Ship the finger server"
    And the home plan is still a regular file
